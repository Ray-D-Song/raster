// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::env;
use std::os::unix::io::FromRawFd;
use std::os::unix::net::UnixStream as StdUnixStream;

use raster_runtime_child_process::install_child_process_ipc;
use raster_runtime_core::modules::module_builder::{ModuleBuilder, ModuleBuilderConfig};
use raster_runtime_core::vm::{Vm, VmOptions};
use rquickjs::{context::EvalOptions, CatchResultExt, Promise};
use tokio::net::UnixStream;
use url::Url;

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let module = env::var("RASTER_FORK_MODULE").map_err(|_| "missing RASTER_FORK_MODULE")?;
    let fd: i32 = env::var("NODE_CHANNEL_FD")
        .map_err(|_| "missing NODE_CHANNEL_FD")?
        .parse()
        .map_err(|_| "invalid NODE_CHANNEL_FD")?;

    let vm = Vm::from_options(VmOptions {
        module_builder: ModuleBuilder::with_config(ModuleBuilderConfig {
            sqlite_enabled: false,
        })
        .with_global(raster_runtime_core::modules::embedded::init)
        .with_global(raster_runtime_core::builtins_inspect::init)
        .with_global(raster_runtime_worker_threads::init)
        .with_module(raster_runtime_core::modules::raster_runtime::hex::RasterRuntimeHexModule)
        .with_module(raster_runtime_core::modules::raster_runtime::qjs::RasterRuntimeQjsModule)
        .with_module(raster_runtime_core::modules::raster_runtime::util::RasterRuntimeUtilModule)
        .with_module(raster_runtime_core::modules::raster_runtime::xml::RasterRuntimeXmlModule),
        ..VmOptions::default()
    })
    .await?;

    let std_stream = unsafe { StdUnixStream::from_raw_fd(fd) };
    std_stream.set_nonblocking(true)?;
    let stream = UnixStream::from_std(std_stream)?;

    vm.ctx
        .async_with(async move |ctx| {
            install_child_process_ipc(&ctx, stream).map_err(|e| e.to_string())?;
            let script_path = resolve_script(&module)?;
            let import_url = Url::from_file_path(&script_path)
                .map_err(|_| format!("Path is not absolute: {script_path}"))?;
            let import_url_lit = format!("{:?}", import_url.as_str());
            let code = format!("await import({import_url_lit});");
            let mut opts = EvalOptions::default();
            opts.promise = true;
            let promise = ctx
                .eval_with_options::<Promise, _>(code, opts)
                .map_err(|e| e.to_string())?;
            if let Err(err) = promise.into_future::<()>().await.catch(&ctx) {
                return Err(err.throw(&ctx).to_string());
            }
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?;

    vm.idle().await?;
    Ok(())
}

fn resolve_script(script: &str) -> Result<String, String> {
    if let Some(path) = script.strip_prefix("file://") {
        return Ok(path.to_string());
    }
    let path = std::path::PathBuf::from(script);
    if path.is_absolute() {
        return Ok(script.to_string());
    }
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    Ok(cwd.join(path).to_string_lossy().into_owned())
}
