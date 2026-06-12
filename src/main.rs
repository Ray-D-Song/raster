use std::{
    fs::File,
    path::{Path, PathBuf},
};

use raster::{
    app::{RasterBundle, RasterRunOptions, path_bundle, run_desktop_raster_app},
    common::utils::logger::{self, LogLevel, LoggerConfig},
    config::{APP_BUNDLE_PATH, DEFAULT_ROOT_HEIGHT, DEFAULT_ROOT_WIDTH},
};

const EMBEDDED_APP_SECTION: &str = "RASTER_APP";

fn main() -> anyhow::Result<()> {
    let options = parse_args()?;
    let log_level = options.effective_log_level();
    logger::init(LoggerConfig {
        level: log_level,
        file_path: options.log_file.clone(),
    })?;
    logger::info("logger initialized");
    match options.command {
        CliCommand::Run { bundle_path } => run_app(bundle_path, false)?,
        CliCommand::Dev { bundle_path } => run_app(Some(bundle_path), true)?,
        CliCommand::Build {
            bundle_path,
            out_path,
            source_exe,
        } => build_executable(&bundle_path, &out_path, source_exe.as_deref())?,
    }
    Ok(())
}

enum CliCommand {
    Run {
        bundle_path: Option<PathBuf>,
    },
    Dev {
        bundle_path: PathBuf,
    },
    Build {
        bundle_path: PathBuf,
        out_path: PathBuf,
        source_exe: Option<PathBuf>,
    },
}

struct CliOptions {
    command: CliCommand,
    log_level: LogLevel,
    log_level_configured: bool,
    log_file: Option<PathBuf>,
}

impl CliOptions {
    fn effective_log_level(&self) -> LogLevel {
        if matches!(self.command, CliCommand::Dev { .. }) && !self.log_level_configured {
            LogLevel::Info
        } else {
            self.log_level
        }
    }
}

fn parse_args() -> anyhow::Result<CliOptions> {
    let mut log_level = LogLevel::default();
    let mut log_level_configured = false;
    let mut log_file = None;
    let mut command_name: Option<String> = None;
    let mut bundle_path: Option<PathBuf> = None;
    let mut out_path: Option<PathBuf> = None;
    let mut source_exe: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "dev" | "build" => {
                if command_name.is_some() {
                    anyhow::bail!("only one command may be specified");
                }
                command_name = Some(arg);
            }
            "--bundle" => {
                bundle_path = Some(PathBuf::from(next_arg(
                    &mut args,
                    "--bundle requires a path",
                )?));
            }
            "--out" => {
                out_path = Some(PathBuf::from(next_arg(&mut args, "--out requires a path")?));
            }
            "--source-exe" => {
                source_exe = Some(PathBuf::from(next_arg(
                    &mut args,
                    "--source-exe requires a path",
                )?));
            }
            "--log-level" => {
                let value = next_arg(&mut args, "--log-level requires info, warn, or error")?;
                log_level = value.parse()?;
                log_level_configured = true;
            }
            "--log-file" => {
                log_file = Some(PathBuf::from(next_arg(
                    &mut args,
                    "--log-file requires a path",
                )?));
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            _ if arg.starts_with("--bundle=") => {
                bundle_path = Some(PathBuf::from(option_value(&arg, "--bundle=")?));
            }
            _ if arg.starts_with("--out=") => {
                out_path = Some(PathBuf::from(option_value(&arg, "--out=")?));
            }
            _ if arg.starts_with("--source-exe=") => {
                source_exe = Some(PathBuf::from(option_value(&arg, "--source-exe=")?));
            }
            _ if arg.starts_with("--log-level=") => {
                let value = option_value(&arg, "--log-level=")?;
                log_level = value.parse()?;
                log_level_configured = true;
            }
            _ if arg.starts_with("--log-file=") => {
                log_file = Some(PathBuf::from(option_value(&arg, "--log-file=")?));
            }
            _ => anyhow::bail!("unknown argument {arg:?}"),
        }
    }

    let command = match command_name.as_deref() {
        Some("dev") => {
            reject_option(out_path.as_ref(), "--out is only supported by build")?;
            reject_option(
                source_exe.as_ref(),
                "--source-exe is only supported by build",
            )?;
            CliCommand::Dev {
                bundle_path: bundle_path
                    .ok_or_else(|| anyhow::anyhow!("dev requires --bundle <path>"))?,
            }
        }
        Some("build") => CliCommand::Build {
            bundle_path: bundle_path
                .ok_or_else(|| anyhow::anyhow!("build requires --bundle <path>"))?,
            out_path: out_path.ok_or_else(|| anyhow::anyhow!("build requires --out <path>"))?,
            source_exe,
        },
        None => {
            reject_option(out_path.as_ref(), "--out requires the build command")?;
            reject_option(
                source_exe.as_ref(),
                "--source-exe requires the build command",
            )?;
            CliCommand::Run { bundle_path }
        }
        Some(command) => anyhow::bail!("unknown command {command:?}"),
    };

    Ok(CliOptions {
        command,
        log_level,
        log_level_configured,
        log_file,
    })
}

fn next_arg(args: &mut impl Iterator<Item = String>, message: &str) -> anyhow::Result<String> {
    args.next()
        .ok_or_else(|| anyhow::anyhow!(message.to_owned()))
}

fn option_value(arg: &str, prefix: &str) -> anyhow::Result<String> {
    let value = arg
        .strip_prefix(prefix)
        .expect("argument should have the expected prefix");
    if value.is_empty() {
        anyhow::bail!("{} requires a value", prefix.trim_end_matches('='));
    }
    Ok(value.to_owned())
}

fn reject_option<T>(value: Option<&T>, message: &str) -> anyhow::Result<()> {
    if value.is_some() {
        anyhow::bail!(message.to_owned());
    }
    Ok(())
}

fn print_help() {
    println!(
        "Usage:\n  raster [--bundle <path>] [--log-level <info|warn|error>] [--log-file <path>]\n  raster dev --bundle <path> [--log-level <info|warn|error>] [--log-file <path>]\n  raster build --bundle <path> --out <path> [--source-exe <path>] [--log-level <info|warn|error>] [--log-file <path>]"
    );
}

fn run_app(bundle_path: Option<PathBuf>, dev_mode: bool) -> anyhow::Result<()> {
    let bundle = if dev_mode {
        path_bundle(
            bundle_path
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("dev requires --bundle <path>"))?,
        )
    } else if let Some(source) = embedded_bundle_source()? {
        RasterBundle::Source {
            name: format!("sui://{EMBEDDED_APP_SECTION}"),
            source,
        }
    } else {
        path_bundle(bundle_path.unwrap_or_else(|| PathBuf::from(APP_BUNDLE_PATH)))
    };

    run_desktop_raster_app(RasterRunOptions {
        width: DEFAULT_ROOT_WIDTH,
        height: DEFAULT_ROOT_HEIGHT,
        bundle,
        dev_mode,
    })
}

fn embedded_bundle_source() -> anyhow::Result<Option<String>> {
    let Some(section) = libsui::find_section(EMBEDDED_APP_SECTION)? else {
        return Ok(None);
    };
    let source = std::str::from_utf8(section)
        .map_err(|error| anyhow::anyhow!("embedded Raster bundle is not valid UTF-8: {error}"))?
        .to_owned();
    Ok(Some(source))
}

fn build_executable(
    bundle_path: &Path,
    out_path: &Path,
    source_exe: Option<&Path>,
) -> anyhow::Result<()> {
    let source_exe = match source_exe {
        Some(path) => path.to_path_buf(),
        None => std::env::current_exe()?,
    };
    let exe = std::fs::read(&source_exe).map_err(|error| {
        anyhow::anyhow!(
            "failed to read source executable {}: {error}",
            source_exe.display()
        )
    })?;
    let bundle = std::fs::read(bundle_path).map_err(|error| {
        anyhow::anyhow!(
            "failed to read Raster bundle {}: {error}",
            bundle_path.display()
        )
    })?;

    if let Some(parent) = out_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| {
            anyhow::anyhow!(
                "failed to create output directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let mut out = File::create(out_path).map_err(|error| {
        anyhow::anyhow!(
            "failed to create output executable {}: {error}",
            out_path.display()
        )
    })?;

    if libsui::utils::is_pe(&exe) {
        libsui::PortableExecutable::from(&exe)?
            .write_resource(EMBEDDED_APP_SECTION, bundle)?
            .build(&mut out)?;
    } else if libsui::utils::is_macho(&exe) {
        libsui::Macho::from(exe)?
            .write_section(EMBEDDED_APP_SECTION, bundle)?
            .build_and_sign(&mut out)?;
    } else if libsui::utils::is_elf(&exe) {
        libsui::Elf::new(&exe).append(EMBEDDED_APP_SECTION, &bundle, &mut out)?;
    } else {
        anyhow::bail!(
            "unsupported source executable format: {}",
            source_exe.display()
        );
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(out_path, std::fs::Permissions::from_mode(0o755)).map_err(
            |error| {
                anyhow::anyhow!(
                    "failed to mark output executable {} as executable: {error}",
                    out_path.display()
                )
            },
        )?;
    }

    logger::info(format!(
        "built Raster executable {} from bundle {}",
        out_path.display(),
        bundle_path.display()
    ));
    Ok(())
}
