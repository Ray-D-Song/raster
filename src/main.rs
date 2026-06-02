mod common;
mod config;
mod gpui_backend;
mod js_runtime;

use std::path::PathBuf;

use common::utils::logger::{self, LogLevel, LoggerConfig};
use config::{APP_BUNDLE_PATH, DEFAULT_ROOT_HEIGHT, DEFAULT_ROOT_WIDTH};

fn main() -> anyhow::Result<()> {
    let options = parse_args()?;
    let log_level = options.effective_log_level();
    logger::init(LoggerConfig {
        level: log_level,
        file_path: options.log_file.clone(),
    })?;
    logger::info("logger initialized");
    let js_runtime = pollster::block_on(js_runtime::start())?;
    if options.dev_mode {
        pollster::block_on(js_runtime.enable_dev_reload())?;
    }
    pollster::block_on(js_runtime.eval_app_bundle_path(&options.bundle_path))?;
    if options.dev_mode {
        pollster::block_on(js_runtime.install_dev_bundle_watcher(&options.bundle_path))?;
    }
    let native_binding = js_runtime.native_binding();
    let runtime_commands = js_runtime.runtime_command_sender();
    js_runtime.spawn_command_loop();

    let dev_reload = options.dev_mode.then_some(gpui_backend::DevReloadConfig {
        demo_bundle_path: options.bundle_path.clone(),
    });
    gpui_backend::start(
        DEFAULT_ROOT_WIDTH,
        DEFAULT_ROOT_HEIGHT,
        dev_reload,
        native_binding,
        runtime_commands,
    );
    Ok(())
}

struct CliOptions {
    dev_mode: bool,
    bundle_path: PathBuf,
    log_level: LogLevel,
    log_level_configured: bool,
    log_file: Option<PathBuf>,
}

impl CliOptions {
    fn effective_log_level(&self) -> LogLevel {
        if self.dev_mode && !self.log_level_configured {
            LogLevel::Info
        } else {
            self.log_level
        }
    }
}

fn parse_args() -> anyhow::Result<CliOptions> {
    let mut dev_mode = false;
    let mut bundle_path = PathBuf::from(APP_BUNDLE_PATH);
    let mut log_level = LogLevel::default();
    let mut log_level_configured = false;
    let mut log_file = None;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dev" => dev_mode = true,
            "--bundle" => {
                let path = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--bundle requires a path"))?;
                bundle_path = PathBuf::from(path);
            }
            "--log-level" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--log-level requires info, warn, or error"))?;
                log_level = value.parse()?;
                log_level_configured = true;
            }
            "--log-file" => {
                let path = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--log-file requires a path"))?;
                log_file = Some(PathBuf::from(path));
            }
            "-h" | "--help" => {
                println!(
                    "Usage: raster [--dev] [--bundle <path>] [--log-level <info|warn|error>] [--log-file <path>]"
                );
                std::process::exit(0);
            }
            _ if arg.starts_with("--bundle=") => {
                let path = arg
                    .strip_prefix("--bundle=")
                    .expect("argument should have --bundle= prefix");
                if path.is_empty() {
                    anyhow::bail!("--bundle requires a path");
                }
                bundle_path = PathBuf::from(path);
            }
            _ if arg.starts_with("--log-level=") => {
                let value = arg
                    .strip_prefix("--log-level=")
                    .expect("argument should have --log-level= prefix");
                if value.is_empty() {
                    anyhow::bail!("--log-level requires info, warn, or error");
                }
                log_level = value.parse()?;
                log_level_configured = true;
            }
            _ if arg.starts_with("--log-file=") => {
                let path = arg
                    .strip_prefix("--log-file=")
                    .expect("argument should have --log-file= prefix");
                if path.is_empty() {
                    anyhow::bail!("--log-file requires a path");
                }
                log_file = Some(PathBuf::from(path));
            }
            _ => anyhow::bail!("unknown argument {arg:?}"),
        }
    }

    Ok(CliOptions {
        dev_mode,
        bundle_path,
        log_level,
        log_level_configured,
        log_file,
    })
}
