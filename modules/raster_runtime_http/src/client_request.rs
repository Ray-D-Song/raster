// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::cell::RefCell;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, Uri};
use raster_runtime_buffer::Buffer;
use raster_runtime_context::CtxExtension;
use raster_runtime_events::{Emitter, EventEmitter, EventList};
use raster_runtime_net::Socket;
use raster_runtime_timers::resume_timer_loop_if_idle;
use raster_runtime_utils::{
    event_loop::{self, EventLoopRef},
    object::ObjectExt,
    result::ResultExt,
    VERSION,
};
use rquickjs::{
    function::{Opt, This},
    prelude::Rest,
    Array, Class, Ctx, Exception, IntoJs, Object, Result, Undefined, Value,
};

use crate::server::{header_value_to_list, IncomingMessage};
use crate::HyperClient;

struct ParsedRequest<'js> {
    uri: Uri,
    method: Method,
    headers: Vec<(String, String)>,
    callback: Option<rquickjs::Function<'js>>,
}

fn hyper_client(ctx: &Ctx<'_>) -> Result<HyperClient> {
    match crate::HTTP_CLIENT.as_ref() {
        Ok(client) => Ok(client.clone()),
        Err(err) => Err(Exception::throw_message(ctx, &err.to_string())),
    }
}

pub fn http_request<'js>(
    ctx: Ctx<'js>,
    args: Rest<Value<'js>>,
) -> Result<Class<'js, ClientRequest<'js>>> {
    let parsed = parse_request_args(&ctx, &args, None)?;
    ClientRequest::start(&ctx, parsed, false)
}

pub fn http_get<'js>(
    ctx: Ctx<'js>,
    args: Rest<Value<'js>>,
) -> Result<Class<'js, ClientRequest<'js>>> {
    let parsed = parse_request_args(&ctx, &args, Some("GET"))?;
    ClientRequest::start(&ctx, parsed, true)
}

fn parse_request_args<'js>(
    ctx: &Ctx<'js>,
    args: &Rest<Value<'js>>,
    force_method: Option<&str>,
) -> Result<ParsedRequest<'js>> {
    let mut url: Option<String> = None;
    let mut options: Option<Object<'js>> = None;
    let mut callback: Option<rquickjs::Function<'js>> = None;

    for value in &args.0 {
        if value.is_function() {
            callback = Some(
                value
                    .as_function()
                    .ok_or_else(|| Exception::throw_type(ctx, "Callback must be a function"))?
                    .clone(),
            );
        } else if value.is_string() {
            url = Some(value.as_string().unwrap().to_string()?);
        } else if value.is_object() {
            options = Some(value.as_object().unwrap().clone());
        }
    }

    let mut method = force_method
        .map(str::to_string)
        .or_else(|| {
            options
                .as_ref()
                .and_then(|o| o.get_optional::<_, String>("method").ok().flatten())
        })
        .unwrap_or_else(|| "GET".into());
    method.make_ascii_uppercase();
    let method = Method::from_bytes(method.as_bytes())
        .map_err(|_| Exception::throw_type(ctx, "Invalid HTTP method"))?;

    let mut headers = Vec::new();
    if let Some(opts) = &options {
        if let Some(hdrs) = opts.get_optional::<_, Object>("headers")? {
            for key in hdrs.keys::<String>() {
                let key = key?;
                let value: Value = hdrs.get(&key)?;
                for part in header_value_to_list(ctx, value)? {
                    headers.push((key.to_ascii_lowercase(), part));
                }
            }
        }
    }

    let uri = if let Some(url) = url {
        url.parse::<Uri>()
            .map_err(|_| Exception::throw_type(ctx, "Invalid URL"))?
    } else {
        build_uri_from_options(ctx, options.as_ref())?
    };

    Ok(ParsedRequest {
        uri,
        method,
        headers,
        callback,
    })
}

fn build_uri_from_options<'js>(ctx: &Ctx<'js>, options: Option<&Object<'js>>) -> Result<Uri> {
    let options = options.ok_or_else(|| Exception::throw_type(ctx, "URL or options required"))?;
    let protocol: String = options
        .get_optional("protocol")?
        .unwrap_or_else(|| "http:".into());
    let default_port = if protocol == "https:" { 443 } else { 80 };
    let port: u16 = options.get_optional("port")?.unwrap_or(default_port);
    let hostname: String = options
        .get_optional::<_, String>("hostname")?
        .or_else(|| options.get_optional("host").ok().flatten())
        .ok_or_else(|| Exception::throw_type(ctx, "hostname is required"))?;
    let hostname = hostname.split(':').next().unwrap_or(&hostname).to_string();
    let path: String = options.get_optional("path")?.unwrap_or_else(|| "/".into());
    let path = if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    };
    let uri = format!("{protocol}//{hostname}:{port}{path}");
    uri.parse::<Uri>()
        .map_err(|_| Exception::throw_type(ctx, "Invalid URL"))
}

#[rquickjs::class]
#[derive(rquickjs::JsLifetime, rquickjs::class::Trace)]
pub struct ClientRequest<'js> {
    emitter: EventEmitter<'js>,
    #[qjs(skip_trace)]
    client: HyperClient,
    #[qjs(skip_trace)]
    uri: Uri,
    #[qjs(skip_trace)]
    method: Method,
    #[qjs(skip_trace)]
    headers: Vec<(String, String)>,
    #[qjs(skip_trace)]
    body: RefCell<Vec<u8>>,
    #[qjs(skip_trace)]
    ended: RefCell<bool>,
    #[qjs(skip_trace)]
    loop_ref: Arc<EventLoopRef>,
}

impl<'js> Emitter<'js> for ClientRequest<'js> {
    fn get_event_list(&self) -> std::sync::Arc<std::sync::RwLock<EventList<'js>>> {
        self.emitter.get_event_list()
    }
}

impl<'js> ClientRequest<'js> {
    fn start(
        ctx: &Ctx<'js>,
        parsed: ParsedRequest<'js>,
        auto_end: bool,
    ) -> Result<Class<'js, Self>> {
        let client = hyper_client(ctx)?;
        let rt_ptr = unsafe { rquickjs::qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
        let rt_key =
            raster_runtime_utils::event_loop::rt_key_from_ptr(rt_ptr as *mut std::ffi::c_void);
        let loop_ref = EventLoopRef::new_referenced(rt_key);

        let instance = Class::instance(
            ctx.clone(),
            Self {
                emitter: EventEmitter::new(),
                client,
                uri: parsed.uri,
                method: parsed.method,
                headers: parsed.headers,
                body: RefCell::new(Vec::new()),
                ended: RefCell::new(false),
                loop_ref,
            },
        )?;

        if let Some(cb) = parsed.callback {
            Self::add_event_listener_str(instance.clone(), ctx, "response", cb, false, false)?;
        }

        if auto_end {
            ClientRequest::end(This(instance.clone()), ctx.clone(), Opt(None))?;
        }

        Ok(instance)
    }

    async fn dispatch(ctx: Ctx<'js>, this: Class<'js, Self>) -> Result<()> {
        let (client, uri, method, headers, body, rt_key) = {
            let borrow = this.borrow();
            let request_body = borrow.body.borrow().clone();
            (
                borrow.client.clone(),
                borrow.uri.clone(),
                borrow.method.clone(),
                borrow.headers.clone(),
                request_body,
                borrow.loop_ref.rt(),
            )
        };

        event_loop::adjust_http_inflight(rt_key, 1);
        let result = Self::dispatch_inner(
            ctx.clone(),
            this.clone(),
            client,
            uri,
            method,
            headers,
            body,
        )
        .await;
        event_loop::adjust_http_inflight(rt_key, -1);
        result
    }

    async fn dispatch_inner(
        ctx: Ctx<'js>,
        this: Class<'js, Self>,
        client: HyperClient,
        uri: Uri,
        method: Method,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<()> {
        let _ = resume_timer_loop_if_idle(&ctx);

        let mut builder = Request::builder().method(method).uri(uri);
        let mut has_user_agent = false;
        for (name, value) in &headers {
            if name.eq_ignore_ascii_case("user-agent") {
                has_user_agent = true;
            }
            builder = builder.header(name.as_str(), value.as_str());
        }
        if !has_user_agent {
            builder = builder.header("user-agent", ["raster_runtime ", VERSION].concat());
        }
        builder = builder.header("accept", "*/*");

        let request = if body.is_empty() {
            builder
                .body(Full::new(Bytes::new()).boxed())
                .or_throw(&ctx)?
        } else {
            builder
                .body(Full::new(Bytes::from(body)).boxed())
                .or_throw(&ctx)?
        };

        let response = client.request(request).await.or_throw(&ctx)?;
        let status = response.status();
        let (parts, incoming_body) = response.into_parts();

        let socket = Socket::new(ctx.clone(), false)?;
        let (headers_obj, raw_headers) = headers_from_parts(&ctx, &parts.headers)?;

        let message = Class::instance(
            ctx.clone(),
            IncomingMessage::from_client_response(
                &ctx,
                status.as_u16(),
                status.canonical_reason().map(str::to_string),
                headers_obj,
                raw_headers,
                socket,
            )?,
        )?;

        ClientRequest::emit_str(
            this.clone(),
            &ctx,
            "response",
            vec![message.clone().into_value()],
            false,
        )?;

        let collected = incoming_body.collect().await.or_throw(&ctx)?;
        let bytes = collected.to_bytes();
        if !bytes.is_empty() {
            message.borrow_mut().append_body(&bytes);
            let chunk = Buffer(bytes.to_vec())
                .into_js(&ctx)
                .unwrap_or_else(|_| Undefined.into_value(ctx.clone()));
            IncomingMessage::emit_str(message.clone(), &ctx, "data", vec![chunk], false)?;
        }
        message.borrow_mut().mark_complete();
        IncomingMessage::emit_str(message, &ctx, "end", vec![], false)?;
        ClientRequest::emit_str(this.clone(), &ctx, "close", vec![], false)?;
        this.borrow().loop_ref.unreference();
        Ok(())
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> ClientRequest<'js> {
    pub fn write(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        chunk: Value<'js>,
        _encoding: Opt<Value<'js>>,
        _cb: Opt<rquickjs::Function<'js>>,
    ) -> Result<bool> {
        if *this.borrow().ended.borrow() {
            return Err(Exception::throw_type(&ctx, "Request already ended"));
        }
        let bytes = chunk_to_bytes(&ctx, chunk)?;
        this.borrow().body.borrow_mut().extend_from_slice(&bytes);
        Ok(true)
    }

    pub fn end(this: This<Class<'js, Self>>, ctx: Ctx<'js>, chunk: Opt<Value<'js>>) -> Result<()> {
        if let Some(chunk) = chunk.0 {
            let bytes = chunk_to_bytes(&ctx, chunk)?;
            this.borrow().body.borrow_mut().extend_from_slice(&bytes);
        }
        if *this.borrow().ended.borrow() {
            return Ok(());
        }
        *this.borrow().ended.borrow_mut() = true;
        let req = this.clone();
        let ctx2 = ctx.clone();
        ctx.spawn_exit_simple(async move {
            ClientRequest::dispatch(ctx2, req).await?;
            Ok(())
        });
        Ok(())
    }

    pub fn abort(this: This<Class<'js, Self>>) -> Class<'js, Self> {
        *this.borrow().ended.borrow_mut() = true;
        this.0
    }
}

fn chunk_to_bytes<'js>(ctx: &Ctx<'js>, chunk: Value<'js>) -> Result<Vec<u8>> {
    if let Some(s) = chunk.as_string() {
        return Ok(s.to_string()?.into_bytes());
    }
    if let Ok(buf) = rquickjs::TypedArray::<u8>::from_value(chunk.clone()) {
        return Ok(buf.as_bytes().map(|b| b.to_vec()).unwrap_or_default());
    }
    if let Some(obj) = chunk.as_object() {
        if let Some(ab) = obj.as_array_buffer() {
            return Ok(ab.as_bytes().map(|b| b.to_vec()).unwrap_or_default());
        }
    }
    Err(Exception::throw_type(ctx, "Chunk must be string or Buffer"))
}

fn headers_from_parts<'js>(
    ctx: &Ctx<'js>,
    headers: &hyper::HeaderMap,
) -> Result<(Object<'js>, Vec<String>)> {
    let headers_obj = Object::new(ctx.clone())?;
    let mut raw = Vec::new();
    let mut grouped: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (name, value) in headers {
        let value = value.to_str().unwrap_or("").to_string();
        let name = name.as_str().to_string();
        raw.push(name.clone());
        raw.push(value.clone());
        grouped.entry(name).or_default().push(value);
    }
    for (name, values) in grouped {
        if name == "set-cookie" {
            let cookies = Array::new(ctx.clone())?;
            for (index, value) in values.into_iter().enumerate() {
                cookies.set(index, value)?;
            }
            headers_obj.set(name, cookies)?;
        } else {
            let separator = if name == "cookie" { "; " } else { ", " };
            headers_obj.set(name, values.join(separator))?;
        }
    }
    Ok((headers_obj, raw))
}
