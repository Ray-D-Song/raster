// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use raster_runtime_events::{Emitter, EventEmitter};
use raster_runtime_net::{Server as NetServer, Socket as NetSocket};
use raster_runtime_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Class, Ctx, Exception, IntoJs, Object, Result, Undefined, Value,
};

use crate::alpn::convert_alpn_protocols_export;
use crate::backend::supported_ciphers;
use crate::client::connect;
use crate::identity::check_server_identity;
use crate::secure_context::{create_js_secure_context, JsSecureContext};
use crate::server::{create_server, Server};
use crate::tls_socket::TlsSocket;

const CLIENT_RENEG_LIMIT: i32 = 3;
const CLIENT_RENEG_WINDOW: i32 = 600;
const DEFAULT_ECDH_CURVE: &str = "auto";

fn link_prototypes<'js>(ctx: &Ctx<'js>, _default: &Object<'js>) -> Result<()> {
    if let (Some(tls_proto), Some(net_proto)) = (
        Class::<TlsSocket>::prototype(ctx)?,
        Class::<NetSocket>::prototype(ctx)?,
    ) {
        tls_proto.set_prototype(Some(&net_proto))?;
    }
    if let (Some(tls_proto), Some(net_proto)) = (
        Class::<Server>::prototype(ctx)?,
        Class::<NetServer>::prototype(ctx)?,
    ) {
        tls_proto.set_prototype(Some(&net_proto))?;
    }
    Ok(())
}

fn default_ciphers() -> String {
    supported_ciphers().join(":")
}

fn check_server_identity_export<'js>(
    ctx: Ctx<'js>,
    hostname: String,
    cert: Object<'js>,
) -> Result<Value<'js>> {
    let cert_obj = crate::certificate::cert_object_from_js(&ctx, &cert)
        .map_err(|e| Exception::throw_message(&ctx, &e))?;
    match check_server_identity(&ctx, &hostname, &cert_obj)? {
        Some(err) => Ok(err),
        None => Undefined.into_js(&ctx),
    }
}

pub struct TlsModule;

impl ModuleDef for TlsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("connect")?;
        declare.declare("createServer")?;
        declare.declare("createSecureContext")?;
        declare.declare("checkServerIdentity")?;
        declare.declare("getCiphers")?;
        declare.declare("convertALPNProtocols")?;
        declare.declare("CLIENT_RENEG_LIMIT")?;
        declare.declare("CLIENT_RENEG_WINDOW")?;
        declare.declare("DEFAULT_CIPHERS")?;
        declare.declare("DEFAULT_ECDH_CURVE")?;
        declare.declare("DEFAULT_MIN_VERSION")?;
        declare.declare("DEFAULT_MAX_VERSION")?;
        declare.declare("TLSSocket")?;
        declare.declare("Server")?;
        declare.declare("SecureContext")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            Class::<JsSecureContext>::define(default)?;
            Class::<TlsSocket>::define(default)?;
            Class::<Server>::define(default)?;

            if Class::<EventEmitter>::prototype(ctx)?.is_some() {
                TlsSocket::add_event_emitter_prototype(ctx)?;
                Server::add_event_emitter_prototype(ctx)?;
            }

            if Class::<NetSocket>::prototype(ctx)?.is_some() {
                link_prototypes(ctx, default)?;
            }

            default.set("connect", Func::from(connect))?;
            default.set("createServer", Func::from(create_server))?;
            default.set(
                "createSecureContext",
                Func::from(|ctx: Ctx<'js>, options: Value<'js>| {
                    create_js_secure_context(&ctx, options)
                }),
            )?;
            default.set(
                "checkServerIdentity",
                Func::from(check_server_identity_export),
            )?;
            default.set(
                "getCiphers",
                Func::from(|_ctx: Ctx<'js>| supported_ciphers()),
            )?;
            default.set(
                "convertALPNProtocols",
                Func::from(|ctx: Ctx<'js>, protocols: Value<'js>, out: Object<'js>| {
                    convert_alpn_protocols_export(&ctx, protocols, &out)
                }),
            )?;

            default.set("CLIENT_RENEG_LIMIT", CLIENT_RENEG_LIMIT)?;
            default.set("CLIENT_RENEG_WINDOW", CLIENT_RENEG_WINDOW)?;
            default.set("DEFAULT_CIPHERS", default_ciphers())?;
            default.set("DEFAULT_ECDH_CURVE", DEFAULT_ECDH_CURVE)?;
            default.set("DEFAULT_MIN_VERSION", "TLSv1.2")?;
            default.set("DEFAULT_MAX_VERSION", "TLSv1.3")?;

            Ok(())
        })?;
        Ok(())
    }
}

impl From<TlsModule> for ModuleInfo<TlsModule> {
    fn from(val: TlsModule) -> Self {
        ModuleInfo {
            name: "tls",
            module: val,
        }
    }
}
