// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use raster_runtime_events::Emitter;
use raster_runtime_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Class, Ctx, Result,
};

pub use self::config::*;
#[cfg(any(
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-graviola",
    feature = "tls-openssl"
))]
mod client_request;
pub use self::server::HttpModule;

#[cfg(any(
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-graviola",
    feature = "tls-openssl"
))]
mod client;
mod config;
mod server;

#[cfg(any(
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-graviola",
    feature = "tls-openssl"
))]
mod agent;

#[cfg(any(
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-graviola",
    feature = "tls-openssl"
))]
pub use self::{agent::Agent, client::*};

pub struct HttpsModule;

impl ModuleDef for HttpsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        #[cfg(any(
            feature = "tls-ring",
            feature = "tls-aws-lc",
            feature = "tls-graviola",
            feature = "tls-openssl"
        ))]
        declare.declare(stringify!(Agent))?;
        declare.declare("createServer")?;
        declare.declare("get")?;
        declare.declare("request")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            #[cfg(any(
                feature = "tls-ring",
                feature = "tls-aws-lc",
                feature = "tls-graviola",
                feature = "tls-openssl"
            ))]
            {
                raster_runtime_buffer::init(ctx)?;
                rquickjs::Class::<Agent>::define(default)?;
                use crate::client_request::{http_get, http_request, ClientRequest};
                Class::<ClientRequest>::define(default)?;
                ClientRequest::add_event_emitter_prototype(ctx)?;
                default.set("request", Func::from(http_request))?;
                default.set("get", Func::from(http_get))?;
            }

            let stub: rquickjs::Function = ctx.eval(
                r#"(function(){
  return function httpsStub() {
    throw new Error("https.createServer is not implemented in raster_runtime yet");
  };
})()"#,
            )?;
            default.set("createServer", stub)?;

            Ok(())
        })
    }
}

impl From<HttpsModule> for ModuleInfo<HttpsModule> {
    fn from(val: HttpsModule) -> Self {
        ModuleInfo {
            name: "https",
            module: val,
        }
    }
}
