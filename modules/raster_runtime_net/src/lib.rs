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
    prelude::{Func, This},
    Class, Ctx, IntoJs, Result, Value,
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

            Socket::add_event_emitter_prototype(ctx)?;
            Server::add_event_emitter_prototype(ctx)?;

            let connect = Func::from(|ctx, args| {
                struct Args<'js>(Ctx<'js>);
                let Args(ctx) = Args(ctx);
                let this = Socket::new(ctx.clone(), false)?;
                Socket::connect(This(this), ctx.clone(), args)
            })
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
