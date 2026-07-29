// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::{Arc, Mutex};

use raster_runtime_context::CtxExtension;
use raster_runtime_events::Emitter;
use raster_runtime_net::{check_network_access, get_hostname, PrefixedTcpStream, Socket};
use raster_runtime_stream::SteamEvents;
use raster_runtime_utils::{error::ErrorExtensions, result::ResultExt};
use rquickjs::{
    prelude::Rest,
    Class, Ctx, Error, Exception, Result, Value,
};
use tokio::net::TcpStream;

use crate::backend::{connect as tls_connect, ConnectOptions, VerifyRecord};
use crate::connect_args::normalize_connect_args;
use crate::error::throw_invalid_arg_value;
use crate::options::parse_connect_options;
use crate::secure_context::SecureContext;
use crate::tls_socket::{rw_join, TlsSocket};

const LOCALHOST: &str = "localhost";

fn handle_connect_error<'js>(
    ctx: &Ctx<'js>,
    tls_socket: Class<'js, TlsSocket<'js>>,
    err: Error,
) -> Result<()> {
    let err_value = err.into_value(ctx)?;
    TlsSocket::fail_connect(tls_socket, ctx, err_value)
}

pub fn connect<'js>(
    ctx: Ctx<'js>,
    args: Rest<Value<'js>>,
) -> Result<Class<'js, TlsSocket<'js>>> {
    let (options_value, callback) = normalize_connect_args(&ctx, args.0)?;
    let options = parse_connect_options(&ctx, options_value.clone())?;

    let check_server_identity_cb = if let Some(obj) = options_value.as_object() {
        obj.get::<_, Option<Value>>("checkServerIdentity")?
            .and_then(|v| v.into_function())
    } else {
        None
    };

    let socket_opt = if let Some(obj) = options_value.as_object() {
        obj.get::<_, Option<Value>>("socket")?
    } else {
        None
    };

    let connect_host = options
        .host
        .clone()
        .unwrap_or_else(|| String::from(LOCALHOST));
    let identity_host = options
        .servername
        .clone()
        .unwrap_or_else(|| connect_host.clone());

    if let Some(ref servername) = options.servername {
        if servername.parse::<std::net::IpAddr>().is_ok() {
            return Err(throw_invalid_arg_value(
                &ctx,
                "servername",
                "must not be an IP address",
            ));
        }
    }

    let sni_name = if let Some(ref servername) = options.servername {
        Some(servername.clone())
    } else if connect_host.parse::<std::net::IpAddr>().is_ok() {
        None
    } else {
        Some(connect_host.clone())
    };

    let context = if let Some(ctx) = options.secure_context.clone() {
        ctx
    } else {
        SecureContext::from_options(&options).map_err(|e| Exception::throw_message(&ctx, &e))?
    };

    let reject_unauthorized = options.reject_unauthorized;
    let timeout = options.timeout;
    let port = options.port;
    let allow_half_open = false;

    let verify_record = if !reject_unauthorized {
        Some(Arc::new(Mutex::new(VerifyRecord {
            ok: true,
            error: None,
        })))
    } else {
        None
    };

    let tls_socket = TlsSocket::new(ctx.clone(), allow_half_open)?;
    {
        let mut borrow = tls_socket.borrow_mut();
        borrow.servername = sni_name.clone();
        borrow.check_server_identity_cb = check_server_identity_cb;
        borrow.secure_connecting = true;
        borrow.pending = true;
    }

    if let Some(cb) = callback {
        TlsSocket::add_event_listener_str(
            tls_socket.clone(),
            &ctx,
            "secureConnect",
            cb,
            true,
            true,
        )?;
    }

    let tls_socket2 = tls_socket.clone();
    let tls_for_error = tls_socket.clone();
    let ctx2 = ctx.clone();
    let identity_host_for_handshake = identity_host.clone();

    ctx.spawn_exit(async move {
        let connect_result = async {
            if let Some(socket_val) = socket_opt {
                let net_socket = Class::<Socket>::from_value(&socket_val)
                    .map_err(|_| Exception::throw_type(&ctx2, "socket must be a net.Socket"))?;

                let (read_half, write_half, prefix) =
                    Socket::begin_tls_handoff(&ctx2, net_socket.clone()).await?;

                let addresses = net_socket.borrow();
                let mut tls_borrow = tls_socket2.borrow_mut();
                tls_borrow.remote_address = addresses.remote_address().clone();
                tls_borrow.remote_port = addresses.remote_port();
                tls_borrow.remote_family = addresses.remote_family().clone();
                tls_borrow.local_address = addresses.local_address().clone();
                tls_borrow.local_port = addresses.local_port();
                tls_borrow.local_family = addresses.local_family().clone();
                drop(tls_borrow);
                drop(addresses);

                TlsSocket::mark_tcp_connected(&tls_socket2, &ctx2)?;

                let io = PrefixedTcpStream::new(prefix, read_half, write_half);
                let connect_opts = ConnectOptions {
                    context,
                    identity_name: identity_host_for_handshake.clone(),
                    sni_name: sni_name.clone(),
                    reject_unauthorized,
                    timeout,
                    verify_record: verify_record.clone(),
                };
                let tls_stream = tls_connect(io, connect_opts).await.map_err(|e| {
                    Exception::throw_message(&ctx2, &e.to_string())
                })?;

                let tls_stream = match TlsSocket::finish_client_handshake(
                    tls_socket2.clone(),
                    ctx2.clone(),
                    tls_stream,
                    reject_unauthorized,
                    &identity_host_for_handshake,
                    verify_record.clone(),
                )
                .await
                {
                    Ok(stream) => stream,
                    Err(err_value) => {
                        TlsSocket::fail_connect(tls_socket2.clone(), &ctx2, err_value)?;
                        return Ok(());
                    }
                };

                let (reader, writer) = tokio::io::split(tls_stream);
                let (readable_done, writable_done) =
                    TlsSocket::process_split_io(&tls_socket2, &ctx2, reader, writer, allow_half_open)?;

                let had_error = rw_join(&ctx2, readable_done, writable_done).await?;
                TlsSocket::emit_close(tls_socket2, &ctx2, had_error)?;
                Ok(())
            } else if let Some(port) = port {
                check_network_access(&ctx2, &connect_host, port)?;
                let addr = get_hostname(&connect_host, port);
                let tcp = TcpStream::connect(addr).await.or_throw(&ctx2)?;
                TlsSocket::set_addresses_from_tcp(&tls_socket2, &ctx2, &tcp)?;
                TlsSocket::mark_tcp_connected(&tls_socket2, &ctx2)?;

                let connect_opts = ConnectOptions {
                    context,
                    identity_name: identity_host_for_handshake.clone(),
                    sni_name: sni_name.clone(),
                    reject_unauthorized,
                    timeout,
                    verify_record: verify_record.clone(),
                };
                let tls_stream = tls_connect(tcp, connect_opts).await.map_err(|e| {
                    Exception::throw_message(&ctx2, &e.to_string())
                })?;

                let tls_stream = match TlsSocket::finish_client_handshake(
                    tls_socket2.clone(),
                    ctx2.clone(),
                    tls_stream,
                    reject_unauthorized,
                    &identity_host_for_handshake,
                    verify_record,
                )
                .await
                {
                    Ok(stream) => stream,
                    Err(err_value) => {
                        TlsSocket::fail_connect(tls_socket2.clone(), &ctx2, err_value)?;
                        return Ok(());
                    }
                };

                let (reader, writer) = tokio::io::split(tls_stream);
                let (readable_done, writable_done) =
                    TlsSocket::process_split_io(&tls_socket2, &ctx2, reader, writer, allow_half_open)?;

                let had_error = rw_join(&ctx2, readable_done, writable_done).await?;
                TlsSocket::emit_close(tls_socket2, &ctx2, had_error)?;
                Ok(())
            } else {
                Err(Exception::throw_type(&ctx2, "port or socket is required"))
            }
        }
        .await;

        if let Err(err) = connect_result {
            handle_connect_error(&ctx2, tls_for_error, err)?;
        }
        Ok(())
    })?;

    Ok(tls_socket)
}
