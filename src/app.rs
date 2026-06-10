use std::path::{Path, PathBuf};

use crate::{
    common::{channel::RuntimeCommand, utils::logger},
    js_runtime::host::NativeBindingState,
};
#[cfg(not(target_os = "android"))]
use crate::gpui_backend;

pub enum RasterBundle {
    Path(PathBuf),
    Source { name: String, source: String },
}

pub struct RasterRunOptions {
    pub width: u32,
    pub height: u32,
    pub bundle: RasterBundle,
    pub dev_mode: bool,
}

pub struct PreparedRasterApp {
    pub native_binding: NativeBindingState,
    pub runtime_commands: crate::common::channel::ChannelSender<RuntimeCommand>,
}

pub async fn prepare_raster_app(options: &RasterRunOptions) -> anyhow::Result<PreparedRasterApp> {
    let js_runtime = crate::js_runtime::start().await?;
    if options.dev_mode {
        js_runtime.enable_dev_reload().await?;
    }

    match &options.bundle {
        RasterBundle::Path(path) => {
            js_runtime.eval_app_bundle_path(path).await?;
            if options.dev_mode {
                js_runtime.install_dev_bundle_watcher(path).await?;
            }
        }
        RasterBundle::Source { name, source } => {
            if options.dev_mode {
                logger::warn("dev reload is ignored for in-memory Raster bundles");
            }
            js_runtime
                .eval_app_bundle_source(name, source.clone())
                .await?;
        }
    }

    let native_binding = js_runtime.native_binding();
    let runtime_commands = js_runtime.runtime_command_sender();
    js_runtime.spawn_command_loop();

    Ok(PreparedRasterApp {
        native_binding,
        runtime_commands,
    })
}

#[cfg(not(target_os = "android"))]
pub fn run_desktop_raster_app(options: RasterRunOptions) -> anyhow::Result<()> {
    let prepared = pollster::block_on(prepare_raster_app(&options))?;
    let dev_reload = match &options.bundle {
        RasterBundle::Path(path) if options.dev_mode => Some(gpui_backend::DevReloadConfig {
            demo_bundle_path: path.clone(),
        }),
        _ => None,
    };

    gpui_backend::start_desktop(
        options.width,
        options.height,
        dev_reload,
        prepared.native_binding,
        prepared.runtime_commands,
    );
    Ok(())
}

pub fn path_bundle(path: impl AsRef<Path>) -> RasterBundle {
    RasterBundle::Path(path.as_ref().to_path_buf())
}
