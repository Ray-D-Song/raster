// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use raster_runtime_core::modules::module_builder::{ModuleBuilder, ModuleBuilderConfig};
use raster_runtime_core::vm::{Vm, VmOptions};
use raster_runtime_utils::clone_wire::{deserialize, serialize};
use raster_runtime_worker_threads::{
    set_worker_isolate_runner, set_worker_thread_id, MainToWorker, WorkerIsolateStart, WorkerToMain,
};
use rquickjs::{function::Opt, prelude::Func, CatchResultExt, Ctx, Function, Promise, Value};

use url::Url;

pub fn register() {
    set_worker_isolate_runner(run_worker_isolate);
}

fn run_worker_isolate(start: WorkerIsolateStart) {
    set_worker_thread_id(start.thread_id);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("worker tokio runtime");
    if let Err(err) = rt.block_on(run_worker_isolate_async(start)) {
        eprintln!("worker isolate error: {err}");
    }
}

async fn run_worker_isolate_async(start: WorkerIsolateStart) -> Result<(), String> {
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
    .await
    .map_err(|e| e.to_string())?;

    let to_main = start.to_main.clone();
    let from_main = start.from_main;

    vm.ctx
        .async_with(async move |ctx| {
            raster_runtime_worker_threads::init(&ctx).map_err(|e| e.to_string())?;

            let worker_data = deserialize(&ctx, &start.worker_data).map_err(|e| e.to_string())?;
            let env_data = deserialize(&ctx, &start.env_snapshot).map_err(|e| e.to_string())?;
            ctx.globals()
                .set("__rasterWorkerBootstrapData", worker_data)
                .map_err(|e| e.to_string())?;
            ctx.globals()
                .set("__rasterWorkerEnvironmentSnapshot", env_data)
                .map_err(|e| e.to_string())?;

            raster_runtime_worker_threads::set_worker_parent_channel(to_main.clone());
            ctx.globals()
                .set(
                    "__rasterWorkerParentPostMessage",
                    Func::from(raster_runtime_worker_threads::worker_parent_post_message_hook),
                )
                .map_err(|e| e.to_string())?;

            let script_path = resolve_script(&ctx, &start.script)?;
            let import_url = Url::from_file_path(&script_path)
                .map_err(|_| format!("Path is not absolute: {script_path}"))?;
            let import_url_lit = format!("{:?}", import_url.as_str());

            let code = format!(
                r#"
globalThis.workerData = globalThis.__rasterWorkerBootstrapData;
delete globalThis.__rasterWorkerBootstrapData;
(function () {{
  const snap = globalThis.__rasterWorkerEnvironmentSnapshot;
  if (snap && typeof snap === "object") {{
    const m = (globalThis.__rasterWorkerEnvBootstrap = new Map());
    for (const [k, v] of Object.entries(snap)) m.set(k, v);
  }}
  delete globalThis.__rasterWorkerEnvironmentSnapshot;
}})();
await import({import_url_lit});
"#
            );

            let mut opts = rquickjs::context::EvalOptions::default();
            opts.promise = true;
            let promise = ctx
                .eval_with_options::<Promise, _>(code, opts)
                .map_err(|e| e.to_string())?;
            if let Err(err) = promise.into_future::<()>().await.catch(&ctx) {
                return Err(err.throw(&ctx).to_string());
            }

            let _ = to_main.send(WorkerToMain::Online);

            loop {
                match from_main.recv() {
                    Ok(MainToWorker::Terminate) => {
                        let _ = to_main.send(WorkerToMain::Exit {
                            code: 0,
                            signal: None,
                        });
                        break;
                    },
                    Ok(MainToWorker::Message(wire)) => {
                        if let Ok(v) = deserialize(&ctx, &wire) {
                            let _ = ctx
                                .globals()
                                .get::<_, Function>("__rasterWorkerDispatchParentMessage")
                                .and_then(|f| f.call::<_, ()>((v,)));
                        }
                    },
                    Ok(MainToWorker::EnvPatch { key, value }) => {
                        if let (Ok(k), Ok(v)) = (deserialize(&ctx, &key), deserialize(&ctx, &value))
                        {
                            let _ = ctx
                                .globals()
                                .get::<_, Function>("__rasterWorkerApplyEnvPatch")
                                .and_then(|f| f.call::<_, ()>((k, v)));
                        }
                    },
                    Err(_) => break,
                }
            }
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?;

    vm.idle().await.map_err(|e| e.to_string())?;
    vm.shutdown().await;
    Ok(())
}

fn resolve_script(_ctx: &Ctx<'_>, script: &str) -> Result<String, String> {
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
