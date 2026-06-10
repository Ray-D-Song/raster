use std::ffi::CString;

use android_activity::AndroidApp;
use gpui::{App, Application, WindowOptions};
use gpui_mobile::android::jni;

use crate::{
    app::{RasterBundle, RasterRunOptions, prepare_raster_app},
    common::utils::logger::{self, LogLevel, LoggerConfig},
    config::{DEFAULT_ROOT_HEIGHT, DEFAULT_ROOT_WIDTH},
    gpui_backend,
};

const ANDROID_BUNDLE_ASSET: &str = "raster/app.js";

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("raster"),
    );
    jni::install_panic_hook();

    let _ = logger::init(LoggerConfig {
        level: LogLevel::Info,
        file_path: None,
    });
    logger::info("android_main entered");

    let source = match load_asset_string(&app, ANDROID_BUNDLE_ASSET) {
        Ok(source) => source,
        Err(error) => {
            logger::error(format!("failed to load Android Raster bundle: {error:#}"));
            return;
        }
    };

    let options = RasterRunOptions {
        width: DEFAULT_ROOT_WIDTH,
        height: DEFAULT_ROOT_HEIGHT,
        bundle: RasterBundle::Source {
            name: format!("asset://{ANDROID_BUNDLE_ASSET}"),
            source,
        },
        dev_mode: false,
    };

    let prepared = match pollster::block_on(prepare_raster_app(&options)) {
        Ok(prepared) => prepared,
        Err(error) => {
            logger::error(format!(
                "failed to prepare Raster Android runtime: {error:#}"
            ));
            return;
        }
    };

    let _platform = jni::init_platform(&app);
    let Some(shared_platform) = jni::shared_platform() else {
        logger::error("failed to get GPUI Android shared platform");
        return;
    };

    logger::info("starting GPUI Android application");
    Application::with_platform(shared_platform.into_rc()).run(move |cx: &mut App| {
        logger::info("opening Raster Android window");
        gpui_backend::open_raster_window(
            cx,
            WindowOptions {
                window_bounds: None,
                titlebar: None,
                focus: true,
                show: true,
                ..WindowOptions::default()
            },
            prepared.native_binding.clone(),
            prepared.runtime_commands.clone(),
        );
        cx.activate(true);
    });
}

fn load_asset_string(app: &AndroidApp, path: &str) -> anyhow::Result<String> {
    let asset_manager = app.asset_manager();
    let c_path = CString::new(path)?;
    let mut asset = asset_manager
        .open(&c_path)
        .ok_or_else(|| anyhow::anyhow!("asset not found: {path}"))?;
    let bytes = asset
        .buffer()
        .map_err(|error| anyhow::anyhow!("failed to read asset buffer for {path}: {error}"))?;
    String::from_utf8(bytes.to_vec())
        .map_err(|error| anyhow::anyhow!("asset {path} is not valid UTF-8: {error}"))
}
