// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{env, result::Result as StdResult};

use rquickjs::{
    context::EvalOptions, loader::FileResolver, prelude::Func, AsyncContext, AsyncRuntime,
    CatchResultExt, Ctx, Error, Function, Result, Value,
};

use crate::libs::{
    context::set_spawn_error_handler,
    hooking::HOOKING_MODE,
    json,
    logging::print_error_and_exit,
    numbers,
    utils::{
        clone::structured_clone,
        primordials::{BasePrimordials, Primordial},
        time,
    },
};
use crate::modules::{
    async_hooks::promise_hook_tracker,
    embedded::{loader::EmbeddedLoader, resolver::EmbeddedResolver, BYTECODE_CACHE},
    module_builder::ModuleBuilder,
    package::{loader::PackageLoader, resolver::PackageResolver},
};
use crate::{environment, http, security};

pub struct Vm {
    pub runtime: AsyncRuntime,
    pub ctx: AsyncContext,
}

pub struct VmOptions {
    pub module_builder: ModuleBuilder,
    pub max_stack_size: usize,
    pub gc_threshold_mb: usize,
}

impl Default for VmOptions {
    fn default() -> Self {
        let module_builder = ModuleBuilder::default()
            .with_global(crate::modules::embedded::init)
            .with_global(crate::builtins_inspect::init)
            .with_module(crate::modules::raster_runtime::hex::RasterRuntimeHexModule)
            .with_module(crate::modules::raster_runtime::qjs::RasterRuntimeQjsModule)
            .with_module(crate::modules::raster_runtime::util::RasterRuntimeUtilModule)
            .with_module(crate::modules::raster_runtime::xml::RasterRuntimeXmlModule);

        Self {
            module_builder,
            max_stack_size: 512 * 1024,
            gc_threshold_mb: {
                const DEFAULT_GC_THRESHOLD_MB: usize = 20;

                let gc_threshold_mb: usize =
                    env::var(environment::ENV_RASTER_RUNTIME_GC_THRESHOLD_MB)
                        .map(|threshold| threshold.parse().unwrap_or(DEFAULT_GC_THRESHOLD_MB))
                        .unwrap_or(DEFAULT_GC_THRESHOLD_MB);

                gc_threshold_mb * 1024 * 1024
            },
        }
    }
}

impl Vm {
    pub async fn from_options(
        vm_options: VmOptions,
    ) -> StdResult<Self, Box<dyn std::error::Error + Send + Sync>> {
        time::init();
        http::init()?;
        security::init()?;

        let mut file_resolver = FileResolver::default();
        let mut paths: Vec<&str> = Vec::with_capacity(10);

        paths.push(".");

        if cfg!(debug_assertions) {
            paths.push("bundle");
        } else {
            paths.push("/opt");
        }

        for path in paths.iter() {
            file_resolver.add_path(*path);
        }

        let (module_resolver, module_loader, mut global_attachment) =
            vm_options.module_builder.build();

        // Public embedded bytecode modules (e.g. stream) must be ModuleNames
        // builtins so require() uses load_via_import instead of disk paths.
        // Keep raster_runtime:* private and out of Module.builtinModules.
        for &name in BYTECODE_CACHE.keys() {
            if !name.starts_with("raster_runtime:") {
                global_attachment = global_attachment.add_name(name);
            }
        }

        let resolver = (
            module_resolver,
            EmbeddedResolver,
            PackageResolver,
            file_resolver,
        );
        let loader = (module_loader, EmbeddedLoader, PackageLoader);

        let runtime = AsyncRuntime::new()?;
        runtime.set_max_stack_size(vm_options.max_stack_size).await;
        runtime.set_gc_threshold(vm_options.gc_threshold_mb).await;
        runtime.set_loader(resolver, loader).await;

        let ctx = AsyncContext::full(&runtime).await?;
        ctx.with(|ctx| {
            (|| {
                BasePrimordials::init(&ctx)?;
                global_attachment.attach(&ctx)?;
                self::init(&ctx)?;
                Ok(())
            })()
            .catch(&ctx)
            .unwrap_or_else(|err| print_error_and_exit(&ctx, err));
            Ok::<_, Error>(())
        })
        .await?;

        if HOOKING_MODE.to_owned() {
            runtime.set_promise_hook(Some(promise_hook_tracker())).await;
        }

        // Unhandled promise rejections must not look like a successful idle exit.
        // Defer emit + default handling to the next macrotask so same-turn
        // `.catch()` can cancel before `unhandledRejection` fires (Node-like).
        // `rejectionHandled` is not emitted yet (follow-up compat item).
        runtime
            .set_host_promise_rejection_tracker(Some(Box::new(
                |ctx, promise, reason, is_handled| {
                    let _ = track_promise_rejection(ctx, promise, reason, is_handled);
                },
            )))
            .await;

        Ok(Vm { runtime, ctx })
    }

    pub async fn new() -> StdResult<Self, Box<dyn std::error::Error + Send + Sync>> {
        let vm = Self::from_options(VmOptions::default()).await?;
        Ok(vm)
    }

    pub async fn run_with<F>(&self, f: F)
    where
        F: for<'js> FnOnce(&Ctx<'js>) -> Result<()> + std::marker::Send,
    {
        self.ctx
            .with(|ctx| {
                if let Err(err) = f(&ctx).catch(&ctx) {
                    print_error_and_exit(&ctx, err);
                }
            })
            .await;
    }

    pub async fn run<S: Into<Vec<u8>> + Send>(&self, source: S, strict: bool, global: bool) {
        self.run_with(|ctx| {
            let mut options = EvalOptions::default();
            options.strict = strict;
            options.promise = true;
            options.global = global;
            let _ = ctx.eval_with_options::<Value, _>(source, options)?;
            Ok::<_, Error>(())
        })
        .await;
    }

    pub async fn run_file(&self, filename: impl AsRef<str>, strict: bool, global: bool) {
        // Await the dynamic import so top-level evaluation errors reject this
        // promise (handled below). Nested async work still relies on the host
        // promise rejection tracker for unhandled rejections.
        let source = [
            r#"await import(""#,
            &filename.as_ref().replace('\\', "/"),
            r#"").catch((e) => {console.error(e);process.exit(1)})"#,
        ]
        .concat();

        self.run(source, strict, global).await;
    }

    pub async fn run_bytecode(&self, bytecode: &[u8]) {
        let bytecode = bytecode.to_vec();
        self.ctx
            .async_with(async move |ctx| {
                let result = async {
                    let module = EmbeddedLoader::load_bytecode_module(ctx.clone(), &bytecode)
                        .map_err(|err| {
                            eprintln!("Failed to evaluate module: {err:?}");
                            err
                        })?;
                    let (_module, promise) = module.eval()?;
                    promise.into_future::<()>().await?;
                    Ok::<_, Error>(())
                }
                .await
                .catch(&ctx);

                if let Err(err) = result {
                    print_error_and_exit(&ctx, err);
                }
            })
            .await;
    }

    pub async fn idle(self) -> StdResult<(), Box<dyn std::error::Error + Sync + Send>> {
        self.runtime.idle().await;
        Ok(())
    }
}

fn track_promise_rejection<'js>(
    ctx: Ctx<'js>,
    promise: Value<'js>,
    reason: Value<'js>,
    is_handled: bool,
) -> Result<()> {
    let track = rejection_tracker_fn(&ctx)?;
    track.call::<_, ()>((promise, reason, is_handled))?;
    Ok(())
}

/// Private ctx userdata holding the deferred rejection tracker function.
/// Pending map / schedule state live in the JS closure — not on `globalThis`.
#[derive(rquickjs::JsLifetime)]
struct RejectionTrackerCache<'js> {
    track: Function<'js>,
}

fn rejection_tracker_fn<'js>(ctx: &Ctx<'js>) -> Result<Function<'js>> {
    if let Some(cached) = ctx.userdata::<RejectionTrackerCache>() {
        return Ok(cached.track.clone());
    }

    // Node-like: only emit / default-log after a macrotask, so same-turn `.catch()`
    // can cancel. `rejectionHandled` is intentionally not emitted yet.
    let track: Function = ctx.eval(
        r#"
(() => {
  const pending = new Map();
  let scheduled = false;
  const flush = () => {
    scheduled = false;
    for (const [promise, reason] of pending) {
      pending.delete(promise);
      let handled = false;
      try {
        handled = process.emit("unhandledRejection", reason, promise) === true;
      } catch (_) {}
      if (handled) continue;
      try {
        console.error("UnhandledPromiseRejection:", reason);
      } catch (_) {}
      try {
        process.exitCode = 1;
      } catch (_) {}
    }
  };
  const schedule = () => {
    if (scheduled) return;
    scheduled = true;
    if (typeof setImmediate === "function") setImmediate(flush);
    else if (typeof setTimeout === "function") setTimeout(flush, 0);
    else Promise.resolve().then(flush);
  };
  return (promise, reason, isHandled) => {
    if (isHandled) {
      pending.delete(promise);
      return;
    }
    pending.set(promise, reason);
    schedule();
  };
})()
"#,
    )?;

    let cache = RejectionTrackerCache {
        track: track.clone(),
    };
    let _ = ctx.store_userdata(cache);
    if let Some(cached) = ctx.userdata::<RejectionTrackerCache>() {
        return Ok(cached.track.clone());
    }
    Ok(track)
}

fn init(ctx: &Ctx<'_>) -> Result<()> {
    set_spawn_error_handler(|ctx, err| {
        print_error_and_exit(ctx, err);
    });

    let globals = ctx.globals();

    globals.set("__gc", Func::from(|ctx: Ctx| ctx.run_gc()))?;
    globals.set("global", ctx.globals())?;
    globals.set("self", ctx.globals())?;
    globals.set(
        "structuredClone",
        Func::from(|ctx, value, options| structured_clone(&ctx, value, options)),
    )?;

    numbers::redefine_prototype(ctx)?;
    json::redefine_static_methods(ctx)?;

    Ok(())
}
