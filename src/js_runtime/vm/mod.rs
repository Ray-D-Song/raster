use llrt_core::module::Module;
use llrt_core::vm::{Vm, VmOptions};
use llrt_core::{CatchResultExt, Exception, Function};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::{
    bridge::BridgeEnvelope,
    common::{
        channel::{ChannelSender, RuntimeCommand, RuntimeCommandQueue},
        mount::NodeValue,
        utils::logger,
    },
    config::JS_MAX_STACK_SIZE,
    js_runtime::{
        bundle::install_runtime_bundle as install_runtime_bundle_exports,
        host::{NativeBindingState, install_native_binding, new_native_binding_state},
        module_loader::build_module_builder,
    },
};

mod bridge_loop;

pub struct JsRuntime {
    vm: Vm,
    native_binding: NativeBindingState,
    commands: RuntimeCommandQueue,
    reload_generation: AtomicU64,
}

pub async fn start() -> anyhow::Result<JsRuntime> {
    let vm = Vm::from_options(VmOptions {
        max_stack_size: JS_MAX_STACK_SIZE,
        module_builder: build_module_builder(),
        ..VmOptions::default()
    })
    .await
    .map_err(|error| anyhow::anyhow!("failed to initialize raster_runtime: {error}"))?;

    let native_binding = new_native_binding_state();
    let commands = RuntimeCommandQueue::new();
    install_host_bindings(&vm, native_binding.clone()).await?;
    install_runtime_bundle(&vm).await?;
    logger::info("js_runtime initialize success");
    Ok(JsRuntime {
        vm,
        native_binding,
        commands,
        reload_generation: AtomicU64::new(0),
    })
}

async fn install_host_bindings(vm: &Vm, native_binding: NativeBindingState) -> anyhow::Result<()> {
    vm.ctx
        .with(|ctx| {
            install_native_binding(ctx.clone(), native_binding)
                .catch(&ctx)
                .map_err(|error| anyhow::anyhow!("failed to install native binding: {error:?}"))
        })
        .await
}

async fn install_runtime_bundle(vm: &Vm) -> anyhow::Result<()> {
    vm.ctx
        .with(|ctx| {
            install_runtime_bundle_exports(ctx.clone())
                .catch(&ctx)
                .map_err(|error| anyhow::anyhow!("failed to load runtime bundle: {error:?}"))?;
            Ok::<_, anyhow::Error>(())
        })
        .await
}

impl JsRuntime {
    #[allow(dead_code)]
    pub fn vm(&self) -> &Vm {
        &self.vm
    }

    pub fn native_binding(&self) -> NativeBindingState {
        self.native_binding.clone()
    }

    pub fn runtime_command_sender(&self) -> ChannelSender<RuntimeCommand> {
        self.commands.sender()
    }

    pub fn spawn_bridge_loop(self) {
        let bridge = self.native_binding.bridge();
        let (wake_tx, wake_rx) = mpsc::channel::<()>();
        bridge.set_js_wake(std::sync::Arc::new(bridge_loop::JsBridgeWake::new(
            wake_tx,
        )));

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    logger::error(format!("failed to start JS runtime event loop: {error}"));
                    return;
                }
            };

            runtime.block_on(async move {
                tokio::spawn(self.vm.runtime.drive());
                let mut pending_reload: Option<(std::path::PathBuf, Instant)> = None;
                loop {
                    bridge_loop::drain_bridge_egress(&self, &bridge).await;

                    if let Some((path, deadline)) = pending_reload.as_ref()
                        && Instant::now() >= *deadline
                    {
                        let path = path.clone();
                        pending_reload = None;
                        if let Err(error) = self.reload_app_bundle_path(&path).await {
                            logger::error(format!("failed to reload app bundle: {error}"));
                        }
                    }

                    let mut woke = false;
                    match self.commands.try_recv() {
                        Ok(RuntimeCommand::Shutdown) => break,
                        Ok(RuntimeCommand::ReloadAppBundle { path }) => {
                            pending_reload =
                                Some((path, Instant::now() + Duration::from_millis(50)));
                            woke = true;
                        }
                        Ok(RuntimeCommand::ReloadAppBundleSource { name, source }) => {
                            if let Err(error) = self.reload_app_bundle_source(name, source).await {
                                logger::error(format!("failed to reload app bundle: {error}"));
                            }
                            woke = true;
                        }
                        Ok(RuntimeCommand::InvokeEvent { .. })
                        | Ok(RuntimeCommand::InvokeQuery { .. })
                        | Ok(RuntimeCommand::EmitRuntimeEvent { .. }) => {}
                        Err(mpsc::TryRecvError::Empty) => {}
                        Err(mpsc::TryRecvError::Disconnected) => break,
                    }

                    if !woke {
                        match wake_rx.try_recv() {
                            Ok(()) => {}
                            Err(mpsc::TryRecvError::Disconnected) => break,
                            Err(mpsc::TryRecvError::Empty) => {
                                tokio::time::sleep(Duration::from_millis(1)).await;
                            }
                        }
                    }
                }
            });
        });
    }

    pub async fn eval_app_bundle_path(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let source = std::fs::read_to_string(path).map_err(|error| {
            anyhow::anyhow!("failed to read app bundle {}: {error}", path.display())
        })?;
        self.eval_app_bundle_source(&path.display().to_string(), source)
            .await
    }

    pub async fn eval_app_bundle_source(
        &self,
        name: impl Into<String>,
        source: String,
    ) -> anyhow::Result<()> {
        let name = name.into();
        self.vm
            .ctx
            .with(|ctx| {
                Module::evaluate(ctx.clone(), name, source)
                    .and_then(|module| module.finish::<()>())
                    .catch(&ctx)
                    .map_err(|error| anyhow::anyhow!("failed to evaluate app bundle: {error:?}"))
            })
            .await
    }

    pub async fn enable_dev_reload(&self) -> anyhow::Result<()> {
        let sender = self.commands.sender();
        self.vm
            .ctx
            .with(|ctx| {
                let request_reload =
                    Function::new(ctx.clone(), move |ctx: llrt_core::Ctx<'_>, path: String| {
                        RuntimeCommandQueue::enqueue(
                            &sender,
                            RuntimeCommand::ReloadAppBundle { path: path.into() },
                        )
                        .map_err(|error| Exception::throw_message(&ctx, &error.to_string()))
                    })?;
                ctx.globals()
                    .set("__rasterRequestAppReload", request_reload)?;
                ctx.eval::<(), _>("globalThis.__rasterDevReload = true;")
                    .catch(&ctx)
                    .map_err(|error| anyhow::anyhow!("failed to enable dev reload: {error:?}"))
            })
            .await
    }

    pub async fn install_dev_bundle_watcher(&self, path: &Path) -> anyhow::Result<()> {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let directory = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("bundle path {} has no parent", path.display()))?;
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("bundle path {} has no file name", path.display()))?;
        let source = format!(
            r#"
                import {{ watch }} from "node:fs";
                const bundlePath = {bundle_path};
                const bundleDirectory = {bundle_directory};
                const bundleFileName = {bundle_filename};
                globalThis.__rasterDevBundleWatcher?.close?.();
                const watcher = watch(bundleDirectory, (eventType, filename) => {{
                  const changedFileName = filename == null ? null : String(filename);
                  if (
                    changedFileName == null ||
                    changedFileName === bundleFileName ||
                    changedFileName.endsWith("/" + bundleFileName)
                  ) {{
                    globalThis.__rasterRequestAppReload?.(bundlePath);
                  }}
                }});
                watcher.on?.("error", (error) => {{
                  console.error("[raster] dev bundle watcher error", error);
                }});
                globalThis.__rasterDevBundleWatcher = watcher;
            "#,
            bundle_path = serde_json::to_string(&path.display().to_string())?,
            bundle_directory = serde_json::to_string(&directory.display().to_string())?,
            bundle_filename = serde_json::to_string(filename)?,
        );
        logger::info(format!("js_runtime dev watcher start: {}", path.display()));
        self.vm
            .ctx
            .with(|ctx| {
                Module::evaluate(ctx.clone(), "raster:dev-reload-watcher".to_owned(), source)
                    .and_then(|module| module.finish::<()>())
                    .catch(&ctx)
                    .map_err(|error| {
                        anyhow::anyhow!("failed to install dev bundle watcher: {error:?}")
                    })
            })
            .await
    }

    async fn reload_app_bundle_path(&self, path: &Path) -> anyhow::Result<()> {
        logger::info(format!(
            "js_runtime reload app bundle start: {}",
            path.display()
        ));
        self.prepare_dev_reload().await?;
        let generation = self.reload_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let name = format!("{}?reload={generation}", path.display());
        let source = std::fs::read_to_string(path).map_err(|error| {
            anyhow::anyhow!("failed to read app bundle {}: {error}", path.display())
        })?;
        self.eval_app_bundle_source(name, source).await?;
        logger::info(format!(
            "js_runtime reload app bundle success: {}",
            path.display()
        ));
        Ok(())
    }

    async fn reload_app_bundle_source(&self, name: String, source: String) -> anyhow::Result<()> {
        logger::info(format!("js_runtime reload app bundle start: {name}"));
        self.prepare_dev_reload().await?;
        let generation = self.reload_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let reload_name = format!("{name}?reload={generation}");
        self.eval_app_bundle_source(reload_name, source).await?;
        logger::info(format!("js_runtime reload app bundle success: {name}"));
        Ok(())
    }

    async fn prepare_dev_reload(&self) -> anyhow::Result<()> {
        self.vm
            .ctx
            .with(|ctx| {
                ctx.eval::<(), _>("globalThis.__rasterPrepareDevReload?.();")
                    .catch(&ctx)
                    .map_err(|error| {
                        anyhow::anyhow!("failed to clear app before reload: {error:?}")
                    })
            })
            .await?;
        Ok(())
    }

    async fn dispatch_bridge_envelope(&self, envelope: BridgeEnvelope) -> anyhow::Result<()> {
        let json = serde_json::to_string(&crate::bridge::js::bridge_envelope_to_json_value(
            &envelope,
        ))?;
        let script = format!("globalThis.__rasterBridgeDispatch?.(JSON.parse({json:?}));");
        self.vm
            .ctx
            .with(|ctx| {
                ctx.eval::<(), _>(script).catch(&ctx).map_err(|error| {
                    anyhow::anyhow!("failed to dispatch bridge envelope: {error:?}")
                })?;
                while ctx.execute_pending_job() {}
                Ok::<_, anyhow::Error>(())
            })
            .await
    }

    #[allow(dead_code)]
    async fn handle_runtime_command(&self, command: RuntimeCommand) -> anyhow::Result<()> {
        match command {
            RuntimeCommand::InvokeEvent {
                handler_id,
                payload,
            } => {
                let payload_json = serde_json::to_string(&payload.to_json_value())?;
                let payload_arg = serde_json::to_string(&payload_json)?;
                let script = format!(
                    "globalThis.__rasterInvokeHandlerJson?.({}, {});",
                    handler_id.0, payload_arg
                );
                self.vm
                    .ctx
                    .with(|ctx| {
                        ctx.eval::<(), _>(script).catch(&ctx).map_err(|error| {
                            anyhow::anyhow!("failed to invoke JS event handler: {error:?}")
                        })
                    })
                    .await?;
                Ok(())
            }
            RuntimeCommand::InvokeQuery {
                handler_id,
                payload,
                responder,
            } => {
                let payload_json = serde_json::to_string(&payload.to_json_value())?;
                let payload_arg = serde_json::to_string(&payload_json)?;
                let script = format!(
                    "globalThis.__rasterInvokeQueryJson?.({}, {}) ?? \"null\";",
                    handler_id.0, payload_arg
                );
                let result_json = self
                    .vm
                    .ctx
                    .with(|ctx| {
                        ctx.eval::<String, _>(script).catch(&ctx).map_err(|error| {
                            anyhow::anyhow!("failed to invoke JS query handler: {error:?}")
                        })
                    })
                    .await?;
                let result = serde_json::from_str::<serde_json::Value>(&result_json)
                    .map(node_value_from_json)
                    .unwrap_or(NodeValue::Null);
                let _ = responder.send(result);
                Ok(())
            }
            RuntimeCommand::ReloadAppBundle { path } => self.reload_app_bundle_path(&path).await,
            RuntimeCommand::ReloadAppBundleSource { name, source } => {
                self.reload_app_bundle_source(name, source).await
            }
            RuntimeCommand::EmitRuntimeEvent { name, payload } => {
                let name_arg = serde_json::to_string(&name)?;
                let payload_json = serde_json::to_string(&payload.to_json_value())?;
                let payload_arg = serde_json::to_string(&payload_json)?;
                let script = format!(
                    "globalThis.__rasterDispatchRuntimeEventJson?.({}, {}); globalThis.__rasterFlushSyncWork?.();",
                    name_arg, payload_arg
                );
                self.vm
                    .ctx
                    .with(|ctx| {
                        ctx.eval::<(), _>(script).catch(&ctx).map_err(|error| {
                            anyhow::anyhow!("failed to dispatch JS runtime event: {error:?}")
                        })
                    })
                    .await
            }
            RuntimeCommand::Shutdown => Ok(()),
        }
    }

    #[cfg(test)]
    async fn runtime_bundle_loaded(&self) -> bool {
        self.vm
            .ctx
            .with(|ctx| ctx.eval::<bool, _>("typeof globalThis.__RasterBundle === 'object'"))
            .await
            .unwrap_or(false)
    }

    #[cfg(test)]
    async fn eval_runtime_script_to_string(&self, script: &str) -> anyhow::Result<String> {
        self.vm
            .ctx
            .with(|ctx| {
                let result = ctx.eval::<String, _>(script).catch(&ctx).map_err(|error| {
                    anyhow::anyhow!("failed to evaluate runtime script: {error:?}")
                })?;
                while ctx.execute_pending_job() {}
                ctx.run_gc();
                Ok(result)
            })
            .await
    }
}

fn node_value_from_json(value: serde_json::Value) -> NodeValue {
    match value {
        serde_json::Value::Null => NodeValue::Null,
        serde_json::Value::Bool(value) => NodeValue::Bool(value),
        serde_json::Value::Number(value) => NodeValue::Number(value.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(value) => NodeValue::String(value),
        serde_json::Value::Array(items) => {
            NodeValue::Array(items.into_iter().map(node_value_from_json).collect())
        }
        serde_json::Value::Object(entries) => NodeValue::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key, node_value_from_json(value)))
                .collect(),
        ),
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
