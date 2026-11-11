// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    result::Result as StdResult,
    str::FromStr,
};

use raster_runtime_events::Emitter;
use raster_runtime_utils::{
    module::{export_default, ModuleInfo},
    result::ResultExt,
};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Rest, This},
    Class, Ctx, Exception, Function, IntoJs, Object, Result, Value,
};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::oneshot::Receiver,
};

pub use self::security::{
    check_network_access, ensure_access, get_allow_list, get_deny_list, set_allow_list,
    set_deny_list,
};

mod security;
mod server;
mod socket;
mod transport;

pub use self::server::Server;
pub use self::socket::{Socket, TransportState};
pub use self::transport::PrefixedTcpStream;

const LOCALHOST: &str = "localhost";

#[derive(PartialEq)]
#[allow(dead_code)]
enum ReadyState {
    Opening,
    Open,
    Closed,
    ReadOnly,
    WriteOnly,
}

impl ReadyState {
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        String::from(match self {
            ReadyState::Opening => "opening",
            ReadyState::Open => "open",
            ReadyState::Closed => "closed",
            ReadyState::ReadOnly => "readOnly",
            ReadyState::WriteOnly => "writeOnly",
        })
    }
}

enum NetStream {
    Tcp((TcpStream, SocketAddr)),
    #[cfg(unix)]
    Unix((UnixStream, tokio::net::unix::SocketAddr)),
}

impl NetStream {
    async fn process<'js>(
        self,
        socket: &Class<'js, Socket<'js>>,
        ctx: &Ctx<'js>,
        allow_half_open: bool,
    ) -> Result<bool> {
        let (readable_done, writable_done) = match self {
            NetStream::Tcp((stream, _)) => {
                Socket::process_tcp_stream(socket, ctx, stream, allow_half_open)
            },
            #[cfg(unix)]
            NetStream::Unix((stream, _)) => {
                Socket::process_unix_stream(socket, ctx, stream, allow_half_open)
            },
        }?;
        let join_result = rw_join(ctx, readable_done, writable_done).await;
        // Always release the control clone, including when join fails.
        socket.borrow_mut().tcp_control = None;
        let had_error = join_result?;
        Ok(had_error)
    }
}

enum Listener {
    Tcp(TcpListener),
    #[cfg(unix)]
    Unix(UnixListener),
}

impl Listener {
    async fn accept(&self, ctx: &Ctx<'_>) -> Result<NetStream> {
        match self {
            Listener::Tcp(tcp) => tcp
                .accept()
                .await
                .map(|(stream, addr)| NetStream::Tcp((stream, addr)))
                .or_throw(ctx),
            #[cfg(unix)]
            Listener::Unix(unix) => unix
                .accept()
                .await
                .map(|(stream, addr)| NetStream::Unix((stream, addr)))
                .or_throw(ctx),
        }
    }
}

pub fn get_hostname(host: &str, port: u16) -> String {
    [host, itoa::Buffer::new().format(port)].join(":")
}

pub fn get_address_parts(
    ctx: &Ctx,
    addr: StdResult<SocketAddr, std::io::Error>,
) -> Result<(String, u16, String)> {
    let addr = addr.or_throw(ctx)?;
    Ok((
        addr.ip().to_string(),
        addr.port(),
        String::from(if addr.is_ipv4() { "IPv4" } else { "IPv6" }),
    ))
}

async fn rw_join(
    ctx: &Ctx<'_>,
    readable_done: Receiver<bool>,
    writable_done: Receiver<bool>,
) -> Result<bool> {
    let (readable_res, writable_res) = tokio::join!(readable_done, writable_done);
    let had_error = readable_res.or_throw_msg(ctx, "Readable sender dropped")?
        || writable_res.or_throw_msg(ctx, "Writable sender dropped")?;
    Ok(had_error)
}

/// Node-compatible scope / zone-id character set for `ip%zone`.
///
/// Empirically Node accepts ASCII alphanumerics plus `.`, `:`, and `-`
/// (`fe80::1%lo0`, `fe80::1%a:b`, `fe80::1%a.b`, `fe80::1%a-b`) and rejects
/// `_`, `/`, and non-ASCII (e.g. `fe80::1%🚀`).
fn is_valid_ipv6_zone_id(zone: &str) -> bool {
    !zone.is_empty()
        && zone
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b':' | b'-'))
}

/// Node-compatible `net.isIP(input)` → `0 | 4 | 6`.
///
/// Non-strings, empty strings, strings with whitespace, and illegal addresses
/// return `0`. IPv6 zone IDs (`fe80::1%lo0`) are accepted only when:
/// - there is exactly one `%`
/// - the zone id is non-empty and uses the allowed character set
/// - the host part is a valid IPv6 address (IPv4 must not carry a zone id)
pub fn is_ip(value: Value<'_>) -> i32 {
    let Some(js_string) = value.as_string() else {
        return 0;
    };
    let Ok(input) = js_string.to_string() else {
        return 0;
    };
    if input.is_empty() || input.chars().any(|c| c.is_whitespace()) {
        return 0;
    }

    if !input.contains('%') {
        if Ipv4Addr::from_str(&input).is_ok() {
            return 4;
        }
        if Ipv6Addr::from_str(&input).is_ok() {
            return 6;
        }
        return 0;
    }

    // Zone ID form: only IPv6%zone with a single non-empty zone segment.
    let mut parts = input.split('%');
    let Some(host) = parts.next() else {
        return 0;
    };
    let Some(zone) = parts.next() else {
        return 0;
    };
    // Reject empty zone (`fe80::1%`) and multiple `%` (`fe80::1%a%b`).
    if parts.next().is_some() || !is_valid_ipv6_zone_id(zone) {
        return 0;
    }
    // IPv4 with zone is never valid (Node returns 0 for `127.0.0.1%x`).
    if Ipv4Addr::from_str(host).is_ok() {
        return 0;
    }
    if Ipv6Addr::from_str(host).is_ok() {
        return 6;
    }
    0
}

/// JS implementation of `net.Socket`.
///
/// The object users hold is a `Duplex`; the Rust class is only its `_handle`
/// (same shape as Bun, `refs/bun/src/js/node/net.ts`). rquickjs binds native
/// class instances with `JS_NewObjectProtoClass` and a later `JS_SetPrototype`
/// has no effect, so the stream surface has to be built in JS.
///
/// Read path: the native loop pushes into this object and stops when `push`
/// returns false; `_read` resumes it. There is one authority over "keep
/// reading" — `_readableState` — and the internal `ReadableStreamInner.state`
/// is never consulted on this path.
const NET_SOCKET_GLUE_JS: &str = r#"(function () {
  "use strict";

  function build(createHandle) {
    const { Duplex } = require("node:stream");

    class Socket extends Duplex {
      constructor(options) {
        const opts = options || {};
        super({
          allowHalfOpen: opts.allowHalfOpen === true,
          readable: true,
          writable: true,
          // The socket emits its own `close`; Duplex must not add one.
          emitClose: false,
          autoDestroy: true,
          decodeStrings: false,
        });

        this._handle = createHandle(opts);
        if (this._handle && typeof this._handle.setOwner === "function") {
          this._handle.setOwner(this);
        }
        // The native driver emits its own `close` when the transport ends.
        // Remember that so `_destroy` does not emit a second one.
        this._closeEmitted = false;
        this.on("close", () => {
          this._closeEmitted = true;
        });

        this.connecting = false;
        this.pending = true;
        this.allowHalfOpen = opts.allowHalfOpen === true;
        this.bytesRead = 0;
        this.bytesWritten = 0;
        this.timeout = 0;

        // `pause()` stops the native loop; `resume()` must restart it. Node only
        // calls `_read` when it wants data, which never happens if the loop is
        // parked, so wake it explicitly on resume.
        this.on("resume", () => {
          if (this._handle && typeof this._handle.resumeReading === "function") {
            this._handle.resumeReading();
          }
        });
      }

      // --- Duplex read hooks ------------------------------------------------
      // The native loop drives `push` directly as bytes arrive. `_read` only
      // tells the handle "the consumer drained, carry on". Returning from a
      // paused loop happens through `resumeReading`, which wakes the native
      // reader without pulling bytes through a second buffer.

      _read(size) {
        const handle = this._handle;
        if (!handle) return;
        // Data is delivered by the native read loop calling `push`; `_read`
        // only tells that loop "the consumer drained, continue". Pulling here
        // as well would create a second data path.
        if (typeof handle.resumeReading === "function") handle.resumeReading();
      }

      _write(chunk, encoding, callback) {
        const handle = this._handle;
        if (!handle) {
          callback(new Error("Socket is closed"));
          return;
        }
        this.bytesWritten += chunk.length;
        try {
          handle.write(chunk, callback);
        } catch (err) {
          callback(err || new Error("Socket write failed"));
        }
      }

      _final(callback) {
        const handle = this._handle;
        if (handle && typeof handle.end === "function") handle.end();
        callback();
      }

      _destroy(err, callback) {
        if (this._handle) this._handle.destroy(err);
        callback(err);
        // `emitClose: false` above means Duplex adds no `close` of its own. The
        // native side only emits one while it still owns this object, and
        // `handle.destroy()` drops that link first, so emit here. Deferred like
        // Node, so a listener attached right after `destroy()` still sees it.
        if (!this._closeEmitted) {
          this._closeEmitted = true;
          const failed = !!err;
          process.nextTick(() => this.emit("close", failed));
        }
      }

      // The native loop calls this once the transport reaches EOF. `push(null)`
      // alone only ends the readable side for a consumer that reads; a socket
      // with just an `end` listener would otherwise never see it, so resume to
      // flush the buffer. Nothing is left to read at EOF, so this cannot cause
      // unbounded buffering.
      _onEof() {
        if (this._readableState && this._readableState.ended) return;
        this.push(null);
        if (this._readableState && this._readableState.flowing !== true) {
          this.resume();
        }
      }

      // --- public surface ---------------------------------------------------

      connect(...args) {
        // A trailing callback is the `connect` listener. Register it on this
        // Duplex: the handle dispatches events through the JS owner, so a
        // listener stored in the handle's own table would never fire.
        let cb;
        if (typeof args[args.length - 1] === "function") cb = args.pop();
        if (cb) this.once("connect", cb);
        const handle = this._handle;
        if (handle && typeof handle.connect === "function") handle.connect(...args);
        return this;
      }

      address() {
        const handle = this._handle;
        if (!handle) return {};
        return {
          address: handle.localAddress,
          family: handle.localFamily,
          port: handle.localPort,
        };
      }

      setTimeout(msecs, callback) {
        this.timeout = msecs;
        if (typeof callback === "function") this.once("timeout", callback);
        return this;
      }

      setNoDelay(noDelay) {
        if (this._handle && typeof this._handle.setNoDelay === "function") {
          this._handle.setNoDelay(noDelay === undefined ? true : !!noDelay);
        }
        return this;
      }

      setKeepAlive(enable, initialDelay) {
        if (this._handle && typeof this._handle.setKeepAlive === "function") {
          this._handle.setKeepAlive(
            enable === undefined ? false : !!enable,
            initialDelay || 0
          );
        }
        return this;
      }

      ref() {
        if (this._handle && typeof this._handle.ref === "function") this._handle.ref();
        return this;
      }

      unref() {
        if (this._handle && typeof this._handle.unref === "function") this._handle.unref();
        return this;
      }

      destroy(error) {
        // Route through `Duplex.destroy` so `destroyed` flips and `_destroy`
        // tears the handle down; calling `_destroy` directly skipped both.
        return super.destroy(error);
      }

      get readyState() {
        return this._handle ? this._handle.readyState : "closed";
      }
      get localAddress() {
        return this._handle ? this._handle.localAddress : undefined;
      }
      get localPort() {
        return this._handle ? this._handle.localPort : undefined;
      }
      get localFamily() {
        return this._handle ? this._handle.localFamily : undefined;
      }
      get remoteAddress() {
        return this._handle ? this._handle.remoteAddress : undefined;
      }
      get remotePort() {
        return this._handle ? this._handle.remotePort : undefined;
      }
      get remoteFamily() {
        return this._handle ? this._handle.remoteFamily : undefined;
      }
      get destroyed() {
        return this._handle ? this._handle.destroyed : true;
      }
    }

    return Socket;
  }

  return { build };
})()"#;

pub struct NetModule;

impl ModuleDef for NetModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("createConnection")?;
        declare.declare("connect")?;
        declare.declare("createServer")?;
        declare.declare("isIP")?;
        declare.declare(stringify!(Socket))?;
        declare.declare(stringify!(Server))?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            Class::<Socket>::define(default)?;
            Class::<Server>::define(default)?;

            Server::add_event_emitter_prototype(ctx)?;

            // `net.Socket` is a JS `Duplex`; the native class is its `_handle`.
            let handle_ctor = Function::new(ctx.clone(), Socket::handle_factory)?;
            let glue: Object = ctx.eval(NET_SOCKET_GLUE_JS)?;
            let build = glue.get::<_, Function>("build")?;
            let socket_class: Function = build.call((handle_ctor,))?;
            default.set("Socket", socket_class.clone())?;
            // The server constructs accepted sockets through the same class.
            ctx.globals()
                .set("__rasterNetSocketClass", socket_class.clone())?;

            let connect = {
                let socket_class = socket_class.clone();
                Func::from(
                    move |ctx: Ctx<'js>, args: Rest<Value<'js>>| -> Result<Value<'js>> {
                        let ctor = socket_class.clone().into_value();
                        let Some(ctor) = ctor.as_constructor() else {
                            return Err(Exception::throw_message(
                                &ctx,
                                "net.Socket is not a constructor",
                            ));
                        };
                        let instance: Value = ctor.construct(())?;
                        let Some(obj) = instance.as_object() else {
                            return Ok(instance);
                        };
                        if let Some(method) = obj
                            .get::<_, Value>("connect")
                            .ok()
                            .and_then(|v| v.as_function().cloned())
                        {
                            let mut call = rquickjs::function::Args::new(ctx.clone(), args.0.len());
                            call.this(obj.clone())?;
                            for arg in args.0 {
                                call.push_arg(arg)?;
                            }
                            method.call_arg::<Value>(call)?;
                        }
                        Ok(instance)
                    },
                )
            }
            .into_js(ctx)?;

            default.set("createConnection", connect.clone())?;
            default.set("connect", connect)?;
            default.set("isIP", Func::from(is_ip))?;
            default.set(
                "createServer",
                Func::from(|ctx, args| {
                    struct Args<'js>(Ctx<'js>);
                    let Args(ctx) = Args(ctx);
                    Server::new(ctx.clone(), args)
                }),
            )
        })?;
        Ok(())
    }
}

impl From<NetModule> for ModuleInfo<NetModule> {
    fn from(val: NetModule) -> Self {
        ModuleInfo {
            name: "net",
            module: val,
        }
    }
}
