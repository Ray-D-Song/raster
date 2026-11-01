# raster_runtime Modules

raster_runtime Modules is a meta-module of [rquickjs](https://github.com/DelSkayn/rquickjs) modules that can be used independently of raster_runtime. They aim to bring to [quickjs](https://bellard.org/quickjs/) APIs from [Node.js](https://nodejs.org/) and [WinterTC](https://wintertc.org/). You can use this meta-module, but each module is also a unique crate.

raster_runtime is a lightweight JavaScript runtime built on QuickJS and maintained for the Raster project.

## Usage

The package is not available in the crate registry yet, but you can clone the repo and import it as a local path.

Use this script to set everything up:

```bash
cd your_project_dir
git clone https://github.com/Ray-D-Song/raster.git

cd raster
npm i
make js
```

Each module has a feature flag, they are all enabled by default but if you prefer to can decide which one you need.
Check the [Compability matrix](#compatibility-matrix) for the full list.

```toml
[dependencies]
raster_runtime_modules = { path = "raster/raster_runtime_modules", default-features = true } # load from local path
rquickjs = { version = "0.11", features = ["full-async"] }
tokio = { version = "1", features = ["full"] }

```

Once you have enable a module, you can import it in your runtime.

> [!NOTE]
> Some modules currently require that you call an `init` function **before** they evaluated.

```rust
use raster_runtime_modules::buffer;
use rquickjs::{async_with, context::EvalOptions, AsyncContext, AsyncRuntime, Error, Module};


#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let runtime = AsyncRuntime::new()?;
    let context = AsyncContext::full(&runtime).await?;

    async_with!(context => |ctx| {
        buffer::init(&ctx)?;
        let (_module, module_eval) = Module::evaluate_def::<buffer::BufferModule,_>(ctx.clone(), "buffer")?;
        module_eval.into_future::<()>().await?;

        let mut options = EvalOptions::default();
        options.global = false;
        if let Err(Error::Exception) = ctx.eval_with_options::<(), _>(
            r#"
            import { Buffer } from "node:buffer";
            Buffer.alloc(10);
            "#,
            options
        ){
            println!("{:#?}", ctx.catch());
        };

        Ok::<_, Error>(())
    })
    .await?;

    Ok(())
}
```

Using ModuleBuilder makes it even simpler.

```rust
use raster_runtime_modules::module_builder::ModuleBuilder;
use rquickjs::{async_with, context::EvalOptions, AsyncContext, AsyncRuntime, Error, Module};


#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let runtime = AsyncRuntime::new()?;

    let module_builder = ModuleBuilder::default();
    let (module_resolver, module_loader, global_attachment) = module_builder.build();
    runtime.set_loader((module_resolver,), (module_loader,)).await;

    let context = AsyncContext::full(&runtime).await?;

    async_with!(context => |ctx| {
        global_attachment.attach(&ctx)?;

        let mut options = EvalOptions::default();
        options.global = false;
        if let Err(Error::Exception) = ctx.eval_with_options::<(), _>(
            r#"
            import { Buffer } from "node:buffer";
            Buffer.alloc(10);
            "#,
            options
        ){
            println!("{:#?}", ctx.catch());
        };

        Ok::<_, Error>(())
    })
    .await?;

    Ok(())
}

```

## Compatibility matrix

> [!NOTE]
> Only a fraction of the Node.js APIs are supported. Below is a high level overview of partially supported APIs and modules.

| Module / API | Node.js | raster_runtime Modules | Feature | Crate / implementation |
| ------------ | ------- | ---------------------- | ------- | ---------------------- |
| abort | ✔︎ | ✔︎ | `abort` | `raster_runtime_abort` |
| assert / assert/strict | ✔︎ | ⚠️ | `assert` | `raster_runtime_assert` |
| async_hooks | ✔︎ | ⚠️ | `async-hooks` | `raster_runtime_async_hooks` |
| buffer | ✔︎ | ⚠️ | `buffer` | `raster_runtime_buffer` |
| child_process | ✔︎ | ⚠️ | `child-process` | `raster_runtime_child_process` |
| console | ✔︎ | ⚠️ | `console` | `raster_runtime_console` |
| constants | ✔︎ | ⚠️ | `constants` | `raster_runtime_constants` |
| crypto | ✔︎ | ⚠️ | `crypto` | `raster_runtime_crypto` |
| dgram | ✔︎ | ⚠️ | `dgram` | `raster_runtime_dgram` |
| diagnostics_channel | ✔︎ | ⚠️ | `diagnostics-channel` | `raster_runtime_diagnostics_channel` |
| dns / dns/promises | ✔︎ | ⚠️ | `dns` | `raster_runtime_dns` |
| events | ✔︎ | ⚠️ | `events` | `raster_runtime_events` |
| exceptions | ✔︎ | ⚠️ | `exceptions` | `raster_runtime_exceptions` |
| fetch | ✔︎ | ⚠️ | `fetch` | `raster_runtime_fetch` |
| fs / fs/promises | ✔︎ | ⚠️ | `fs` | `raster_runtime_fs` |
| http | ✔︎ | ⚠️ | `http` | `raster_runtime_http` |
| http2 | ✔︎ | ⚠️ | N/A | embedded JS load-time surface; networking methods throw |
| https | ✔︎ | ⚠️ | `https` | `raster_runtime_http` |
| inspector | ✔︎ | ⚠️ | `inspector` | `raster_runtime_inspector` |
| intl | ✔︎ | ⚠️ | `intl` | `raster_runtime_intl` |
| module | ✔︎ | ⚠️ | N/A | built-in CommonJS loader facade |
| navigator | ✔︎ | ⚠️ | `navigator` | `raster_runtime_navigator` |
| net | ✔︎ | ⚠️ | `net` | `raster_runtime_net` |
| os | ✔︎ | ⚠️ | `os` | `raster_runtime_os` |
| path | ✔︎ | ⚠️ | `path` | `raster_runtime_path` |
| perf_hooks | ✔︎ | ⚠️ | `perf-hooks` | `raster_runtime_perf_hooks` |
| process | ✔︎ | ⚠️ | `process` | `raster_runtime_process` |
| querystring | ✔︎ | ⚠️ | `querystring` | `raster_runtime_querystring` |
| readline / readline/promises | ✔︎ | ✔︎ | N/A | embedded JS |
| sqlite | ✔︎ | ⚠️ | `sqlite` | `raster_runtime_sqlite` (Node 24.3 / SQLite 3.50.1) |
| stream / stream/promises | ✔︎ | ✔︎ | N/A | `raster_runtime_stream` / embedded JS |
| stream/web | ✔︎ | ⚠️ | `stream-web` | `raster_runtime_stream_web` |
| string_decoder | ✔︎ | ✔︎ | `string-decoder` | `raster_runtime_string_decoder` |
| temporal | ✔︎ | ⚠️ | `temporal` | `raster_runtime_temporal` |
| timers / timers/promises | ✔︎ | ⚠️ | `timers` | `raster_runtime_timers` |
| tls | ✔︎ | ⚠️ | `tls` | `raster_runtime_tls` |
| tty | ✔︎ | ⚠️ | `tty` | `raster_runtime_tty` |
| url | ✔︎ | ⚠️ | `url` | `raster_runtime_url` |
| util / util/types | ✔︎ | ⚠️ | `util` | `raster_runtime_util` |
| v8 | ✔︎ | ⚠️ | `v8` | `raster_runtime_v8` |
| vm | ✔︎ | ⚠️ | `vm` | `raster_runtime_vm` |
| WebAssembly | ✔︎ | ⚠️ | `webassembly` | `raster_runtime_webassembly` |
| worker_threads | ✔︎ | ⚠️ | N/A | OS-thread isolates; structured-clone postMessage; SAB hooks |
| zlib | ✔︎ | ⚠️ | `zlib` | `raster_runtime_zlib` |
| Other modules | ✔︎ | ✘ | N/A | N/A |

_⚠️ = partially supported_

## License

This module is licensed under the Apache-2.0 License.
