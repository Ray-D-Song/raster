// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::{Arc, Mutex, RwLock};

use raster_runtime_events::{Emitter, EventEmitter, EventKey, EventList};
use raster_runtime_stream::{
    impl_stream_events,
    readable::{ReadableStream, ReadableStreamInner},
    writable::{WritableStream, WritableStreamInner},
    SteamEvents,
};
use raster_runtime_utils::{error::ErrorExtensions, result::ResultExt};
use rquickjs::{
    class::{Trace, Tracer},
    prelude::{Opt, Rest, This},
    Class, Ctx, Exception, Function, IntoJs, JsLifetime, Null, Object, Result, Value,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::oneshot::Receiver;

use crate::backend::{
    inspect_client_connection, inspect_server_connection, ClientTlsStream, ServerTlsStream,
    TlsConnectionInfo, VerifyRecord,
};
use crate::certificate::{parse_cert_chain_der, parse_cert_der, CertObject};

impl_stream_events!(TlsSocket);

#[derive(PartialEq)]
pub(crate) enum TlsRole {
    Client,
    Server,
}

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
    fn to_string(&self) -> String {
        String::from(match self {
            ReadyState::Opening => "opening",
            ReadyState::Open => "open",
            ReadyState::Closed => "closed",
            ReadyState::ReadOnly => "readOnly",
            ReadyState::WriteOnly => "writeOnly",
        })
    }
}

#[rquickjs::class(rename = "TLSSocket")]
pub struct TlsSocket<'js> {
    emitter: EventEmitter<'js>,
    readable_stream_inner: ReadableStreamInner<'js>,
    writable_stream_inner: WritableStreamInner<'js>,
    pub(crate) connecting: bool,
    pub(crate) destroyed: bool,
    pub(crate) pending: bool,
    pub(crate) local_address: Option<String>,
    pub(crate) local_family: Option<String>,
    pub(crate) local_port: Option<u16>,
    pub(crate) remote_address: Option<String>,
    pub(crate) remote_family: Option<String>,
    pub(crate) remote_port: Option<u16>,
    ready_state: ReadyState,
    #[allow(dead_code)]
    allow_half_open: bool,
    pub(crate) secure_connecting: bool,
    pub(crate) authorized: bool,
    pub(crate) authorization_error: Option<Value<'js>>,
    alpn_protocol: Option<String>,
    pub(crate) servername: Option<String>,
    protocol: Option<String>,
    cipher_name: Option<String>,
    cipher_standard_name: Option<String>,
    peer_certs: Vec<CertObject>,
    local_cert_der: Option<Vec<u8>>,
    pub(crate) check_server_identity_cb: Option<Function<'js>>,
}

unsafe impl<'js> JsLifetime<'js> for TlsSocket<'js> {
    type Changed<'to> = TlsSocket<'to>;
}

impl<'js> Trace<'js> for TlsSocket<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.emitter.trace(tracer);
        self.authorization_error.trace(tracer);
        if let Some(cb) = &self.check_server_identity_cb {
            cb.trace(tracer);
        }
    }
}

impl<'js> Emitter<'js> for TlsSocket<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.emitter.get_event_list()
    }

    fn on_event_changed(&mut self, event: EventKey<'js>, added: bool) -> Result<()> {
        self.readable_stream_inner.on_event_changed(event, added)
    }
}

impl<'js> ReadableStream<'js> for TlsSocket<'js> {
    fn inner_mut(&mut self) -> &mut ReadableStreamInner<'js> {
        &mut self.readable_stream_inner
    }

    fn inner(&self) -> &ReadableStreamInner<'js> {
        &self.readable_stream_inner
    }
}

impl<'js> WritableStream<'js> for TlsSocket<'js> {
    fn inner_mut(&mut self) -> &mut WritableStreamInner<'js> {
        &mut self.writable_stream_inner
    }

    fn inner(&self) -> &WritableStreamInner<'js> {
        &self.writable_stream_inner
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> TlsSocket<'js> {
    #[qjs(get, enumerable)]
    pub fn encrypted(&self) -> bool {
        true
    }

    #[qjs(get, enumerable)]
    pub fn secure_connecting(&self) -> bool {
        self.secure_connecting
    }

    #[qjs(get, enumerable)]
    pub fn authorized(&self) -> bool {
        self.authorized
    }

    #[qjs(get, enumerable)]
    pub fn authorization_error(&self) -> Option<Value<'js>> {
        self.authorization_error.clone()
    }

    #[qjs(get, enumerable)]
    pub fn alpn_protocol(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(protocol) = &self.alpn_protocol {
            protocol.clone().into_js(&ctx)
        } else {
            false.into_js(&ctx)
        }
    }

    #[qjs(get, enumerable)]
    pub fn servername(&self) -> Option<String> {
        self.servername.clone()
    }

    #[qjs(get, enumerable)]
    pub fn connecting(&self) -> bool {
        self.connecting
    }

    #[qjs(get, enumerable)]
    pub fn pending(&self) -> bool {
        self.pending
    }

    #[qjs(get, enumerable)]
    pub fn remote_address(&self) -> Option<String> {
        self.remote_address.clone()
    }

    #[qjs(get, enumerable)]
    pub fn local_address(&self) -> Option<String> {
        self.local_address.clone()
    }

    #[qjs(get, enumerable)]
    pub fn remote_family(&self) -> Option<String> {
        self.remote_family.clone()
    }

    #[qjs(get, enumerable)]
    pub fn local_family(&self) -> Option<String> {
        self.local_family.clone()
    }

    #[qjs(get, enumerable)]
    pub fn remote_port(&self) -> Option<u16> {
        self.remote_port
    }

    #[qjs(get, enumerable)]
    pub fn local_port(&self) -> Option<u16> {
        self.local_port
    }

    #[qjs(get, enumerable)]
    pub fn ready_state(&self) -> String {
        self.ready_state.to_string()
    }

    pub fn write(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        value: Value<'js>,
        cb: Opt<Function<'js>>,
    ) -> Result<bool> {
        WritableStream::write_flushed(this, ctx, value, cb)?;
        Ok(true)
    }

    pub fn end(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        args: Rest<Value<'js>>,
    ) -> Result<Class<'js, Self>> {
        let mut args = args.0.into_iter();
        let first = args.next();
        let (value, callback) = match first {
            Some(value) if value.is_function() => (None, value.into_function()),
            Some(value) => (
                Some(value),
                args.next().and_then(|value| value.into_function()),
            ),
            None => (None, None),
        };
        if let Some(cb) = callback {
            Self::add_event_listener_str(this.clone(), &ctx, "finish", cb, true, true)?;
        }
        if let Some(value) = value {
            WritableStream::write_flushed(This(this.0.clone()), ctx.clone(), value, Opt(None))?;
        }
        WritableStream::end(This(this.clone()));
        Ok(this.0)
    }

    pub fn destroy(this: This<Class<'js, Self>>, error: Opt<Value<'js>>) -> Class<'js, Self> {
        this.borrow_mut().destroyed = true;
        ReadableStream::destroy(This(this.clone()), Opt(None));
        WritableStream::destroy(This(this.clone()), error);
        this.0
    }

    pub fn read(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        size: Opt<usize>,
    ) -> Result<Value<'js>> {
        ReadableStream::read(this, ctx, size)
    }

    pub fn get_peer_certificate(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        detailed: Opt<bool>,
    ) -> Result<Value<'js>> {
        let borrow = this.borrow();
        let cert = borrow.peer_certs.first();
        if cert.is_none() {
            return Object::new(ctx.clone())?.into_js(&ctx);
        }
        let cert = cert.unwrap();
        let obj = if detailed.0.unwrap_or(false) {
            cert.to_js_object_detailed(&ctx)?
        } else {
            cert.to_js_object(&ctx)?
        };
        obj.into_js(&ctx)
    }

    pub fn get_certificate(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let borrow = this.borrow();
        if let Some(der) = &borrow.local_cert_der {
            if let Ok(cert) = parse_cert_der(der) {
                return cert.to_js_object(&ctx)?.into_js(&ctx);
            }
        }
        Undefined.into_js(&ctx)
    }

    pub fn get_cipher(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let borrow = this.borrow();
        let Some(name) = &borrow.cipher_name else {
            return Null.into_js(&ctx);
        };
        let obj = Object::new(ctx.clone())?;
        obj.set("name", name.clone())?;
        let standard_name = borrow
            .cipher_standard_name
            .as_deref()
            .unwrap_or(name.as_str());
        obj.set("standardName", standard_name)?;
        let version = borrow
            .protocol
            .as_deref()
            .map(crate::version::normalize_protocol_name)
            .unwrap_or_else(|| "TLSv1.2".to_string());
        obj.set("version", version)?;
        obj.into_js(&ctx)
    }

    pub fn get_protocol(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(protocol) = &this.borrow().protocol {
            protocol.clone().into_js(&ctx)
        } else {
            Null.into_js(&ctx)
        }
    }

    pub fn is_session_reused(_this: This<Class<'js, Self>>) -> bool {
        false
    }
}

impl<'js> TlsSocket<'js> {
    pub fn new(ctx: Ctx<'js>, allow_half_open: bool) -> Result<Class<'js, Self>> {
        let emitter = EventEmitter::new();
        let readable_stream_inner = ReadableStreamInner::new(emitter.clone(), false);
        let writable_stream_inner = WritableStreamInner::new(emitter.clone(), false);

        Class::instance(
            ctx,
            Self {
                emitter,
                connecting: false,
                destroyed: false,
                pending: true,
                ready_state: ReadyState::Closed,
                local_address: None,
                local_family: None,
                local_port: None,
                remote_address: None,
                remote_family: None,
                remote_port: None,
                readable_stream_inner,
                writable_stream_inner,
                allow_half_open,
                secure_connecting: true,
                authorized: false,
                authorization_error: None,
                alpn_protocol: None,
                servername: None,
                protocol: None,
                cipher_name: None,
                cipher_standard_name: None,
                peer_certs: Vec::new(),
                local_cert_der: None,
                check_server_identity_cb: None,
            },
        )
    }

    pub fn apply_connection_info(&mut self, ctx: &Ctx<'js>, info: &TlsConnectionInfo) {
        self.protocol = info
            .protocol
            .as_ref()
            .map(|p| crate::version::normalize_protocol_name(p));
        self.cipher_name = info.cipher.clone();
        self.cipher_standard_name = info
            .cipher_standard_name
            .clone()
            .or_else(|| info.cipher.clone());
        if let Some(alpn) = &info.alpn_protocol {
            self.alpn_protocol = Some(String::from_utf8_lossy(alpn).to_string());
        }
        if !info.peer_certs.is_empty() {
            if let Ok(certs) = parse_cert_chain_der(&info.peer_certs) {
                self.peer_certs = certs;
            }
        }
        self.local_cert_der = info.local_cert.clone();
        if let Some(err) = &info.chain_error {
            if let Ok(exception) = Exception::from_message(ctx.clone(), err) {
                self.authorization_error = Some(exception.into_value());
            }
        }
    }

    pub fn fail_connect(this: Class<'js, Self>, ctx: &Ctx<'js>, error: Value<'js>) -> Result<()> {
        {
            let mut borrow = this.borrow_mut();
            borrow.secure_connecting = false;
            borrow.connecting = false;
            borrow.pending = false;
            borrow.ready_state = ReadyState::Closed;
            borrow.destroyed = true;
        }
        if this.borrow().has_listener_str("error") {
            Self::emit_str(this.clone(), ctx, "error", vec![error], false)?;
            Self::emit_close(this, ctx, true)?;
            Ok(())
        } else {
            Self::emit_close(this.clone(), ctx, true)?;
            Err(Exception::from_value(error)?.throw())
        }
    }

    pub async fn finish_client_handshake<IO>(
        this: Class<'js, Self>,
        ctx: Ctx<'js>,
        stream: ClientTlsStream<IO>,
        reject_unauthorized: bool,
        hostname: &str,
        verify_record: Option<Arc<Mutex<VerifyRecord>>>,
    ) -> std::result::Result<ClientTlsStream<IO>, Value<'js>>
    where
        IO: AsyncRead + AsyncWrite + Send + Unpin + 'js,
    {
        let info = inspect_client_connection(&stream, verify_record.as_ref());
        Self::complete_handshake(
            this,
            &ctx,
            info,
            Some(hostname),
            reject_unauthorized,
            TlsRole::Client,
        )
        .map_err(|err| {
            err.into_value(&ctx)
                .unwrap_or_else(|_| Undefined.into_value(ctx.clone()))
        })?;
        Ok(stream)
    }

    pub async fn finish_server_handshake<IO>(
        this: Class<'js, Self>,
        ctx: Ctx<'js>,
        stream: ServerTlsStream<IO>,
        reject_unauthorized: bool,
        selected_local_cert: Arc<Mutex<Option<Vec<u8>>>>,
    ) -> std::result::Result<ServerTlsStream<IO>, Value<'js>>
    where
        IO: AsyncRead + AsyncWrite + Send + Unpin + 'js,
    {
        let info = inspect_server_connection(&stream, reject_unauthorized, &selected_local_cert);
        Self::complete_handshake(this, &ctx, info, None, reject_unauthorized, TlsRole::Server)
            .map_err(|err| {
                err.into_value(&ctx)
                    .unwrap_or_else(|_| Undefined.into_value(ctx.clone()))
            })?;
        Ok(stream)
    }

    fn complete_handshake(
        this: Class<'js, Self>,
        ctx: &Ctx<'js>,
        info: TlsConnectionInfo,
        identity_hostname: Option<&str>,
        reject_unauthorized: bool,
        role: TlsRole,
    ) -> Result<()> {
        {
            let mut borrow = this.borrow_mut();
            borrow.apply_connection_info(ctx, &info);
        }

        let chain_authorized = info.chain_authorized;

        if !chain_authorized && reject_unauthorized {
            let err_msg = info
                .chain_error
                .unwrap_or_else(|| "certificate verification failed".to_string());
            return Err(Exception::from_message(ctx.clone(), &err_msg)?.throw());
        }

        let mut identity_ok = true;
        let mut identity_error: Option<Value<'js>> = None;

        if role == TlsRole::Client
            && (chain_authorized || !reject_unauthorized)
            && !info.peer_certs.is_empty()
        {
            let hostname = identity_hostname.unwrap_or("");
            if let Ok(certs) = parse_cert_chain_der(&info.peer_certs) {
                if let Some(leaf) = certs.first() {
                    let custom_cb = this.borrow().check_server_identity_cb.clone();
                    if let Some(cb) = custom_cb {
                        let cert_obj = leaf.to_js_object(ctx)?;
                        let result = cb.call::<_, Value>((hostname, cert_obj))?;
                        if !result.is_undefined() && !result.is_null() {
                            identity_ok = false;
                            identity_error = Some(result);
                        }
                    } else if let Some(err) =
                        crate::identity::check_server_identity(ctx, hostname, leaf)?
                    {
                        identity_ok = false;
                        identity_error = Some(err);
                    }
                }
            }
        }

        if !identity_ok && reject_unauthorized {
            return Err(if let Some(err) = identity_error {
                Exception::from_value(err)?.throw()
            } else {
                Exception::from_message(ctx.clone(), "identity check failed")?.throw()
            });
        }

        {
            let mut borrow = this.borrow_mut();
            borrow.authorized = if role == TlsRole::Server {
                chain_authorized
            } else {
                chain_authorized && identity_ok
            };
            if let Some(err) = identity_error {
                borrow.authorization_error = Some(err);
            }
            borrow.secure_connecting = false;
            borrow.connecting = false;
            borrow.pending = false;
            borrow.ready_state = ReadyState::Open;
            if role == TlsRole::Server {
                borrow.servername = info.client_servername.clone();
            }
        }

        if role == TlsRole::Client {
            Self::emit_str(this.clone(), ctx, "secure", vec![], false)?;
            Self::emit_str(this.clone(), ctx, "secureConnect", vec![], false)?;
        }
        Ok(())
    }

    pub fn process_split_io<R, W>(
        this: &Class<'js, Self>,
        ctx: &Ctx<'js>,
        reader: R,
        writer: W,
        allow_half_open: bool,
    ) -> Result<(Receiver<bool>, Receiver<bool>)>
    where
        R: AsyncRead + Send + Unpin + 'js + 'static,
        W: AsyncWrite + Send + Unpin + 'js + 'static,
    {
        let this2 = this.clone();
        let this3 = this.clone();
        let readable_done =
            ReadableStream::process_callback(this.clone(), ctx, reader, move || {
                if !allow_half_open {
                    WritableStream::end(This(this3));
                }
            })?;
        let writable_done = WritableStream::process(this2, ctx, writer)?;
        Ok((readable_done, writable_done))
    }

    pub fn mark_tcp_connected(this: &Class<'js, Self>, ctx: &Ctx<'js>) -> Result<()> {
        let mut borrow = this.borrow_mut();
        borrow.connecting = true;
        borrow.pending = false;
        borrow.ready_state = ReadyState::Opening;
        drop(borrow);
        Self::emit_str(this.clone(), ctx, "connect", vec![], false)?;
        Ok(())
    }

    pub fn set_addresses_from_tcp(
        this: &Class<'js, Self>,
        _ctx: &Ctx<'js>,
        stream: &tokio::net::TcpStream,
    ) -> Result<()> {
        let mut borrow = this.borrow_mut();
        if let Ok(addr) = stream.peer_addr() {
            borrow.remote_address = Some(addr.ip().to_string());
            borrow.remote_port = Some(addr.port());
            borrow.remote_family = Some(if addr.is_ipv4() {
                "IPv4".to_string()
            } else {
                "IPv6".to_string()
            });
        }
        if let Ok(addr) = stream.local_addr() {
            borrow.local_address = Some(addr.ip().to_string());
            borrow.local_port = Some(addr.port());
            borrow.local_family = Some(if addr.is_ipv4() {
                "IPv4".to_string()
            } else {
                "IPv6".to_string()
            });
        }
        drop(borrow);
        Ok(())
    }
}

pub async fn rw_join(
    ctx: &Ctx<'_>,
    readable_done: Receiver<bool>,
    writable_done: Receiver<bool>,
) -> Result<bool> {
    let (readable_res, writable_res) = tokio::join!(readable_done, writable_done);
    let had_error = readable_res.or_throw_msg(ctx, "Readable sender dropped")?
        || writable_res.or_throw_msg(ctx, "Writable sender dropped")?;
    Ok(had_error)
}

use rquickjs::Undefined;
