use std::{
    ffi::CString,
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

use android_activity::AndroidApp;
use gpui::{App, Application, WindowOptions};
use gpui_mobile::android::jni;

use crate::{
    app::{RasterBundle, RasterRunOptions, prepare_raster_app},
    common::{
        channel::{RuntimeCommand, RuntimeCommandQueue},
        utils::logger::{self, LogLevel, LoggerConfig},
    },
    config::{DEFAULT_ROOT_HEIGHT, DEFAULT_ROOT_WIDTH},
    gpui_backend,
};

const ANDROID_BUNDLE_ASSET: &str = "raster/app.js";
const ANDROID_DEV_CONFIG_ASSET: &str = "raster/dev.json";
const ANDROID_DEV_INITIAL_RETRY_COUNT: usize = 25;
const ANDROID_DEV_INITIAL_RETRY_INTERVAL: Duration = Duration::from_millis(200);
const ANDROID_DEV_POLL_INTERVAL: Duration = Duration::from_millis(500);
const ANDROID_DEV_HTTP_TIMEOUT: Duration = Duration::from_millis(800);

struct LoadedAndroidBundle {
    name: String,
    source: String,
    dev_urls: Vec<String>,
    initial_version: Option<String>,
}

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

    let loaded_bundle = match load_android_bundle(&app) {
        Ok(bundle) => bundle,
        Err(error) => {
            logger::error(format!("failed to load Android Raster bundle: {error:#}"));
            return;
        }
    };
    let dev_mode = !loaded_bundle.dev_urls.is_empty();

    let options = RasterRunOptions {
        width: DEFAULT_ROOT_WIDTH,
        height: DEFAULT_ROOT_HEIGHT,
        bundle: RasterBundle::Source {
            name: loaded_bundle.name,
            source: loaded_bundle.source,
        },
        dev_mode,
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
    if dev_mode {
        start_dev_server_reloader(
            loaded_bundle.dev_urls,
            loaded_bundle.initial_version,
            prepared.runtime_commands.clone(),
        );
    }

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

fn load_android_bundle(app: &AndroidApp) -> anyhow::Result<LoadedAndroidBundle> {
    match load_dev_urls(app) {
        Ok(dev_urls) => {
            for attempt in 0..=ANDROID_DEV_INITIAL_RETRY_COUNT {
                for url in &dev_urls {
                    match http_get_text(url, ANDROID_DEV_HTTP_TIMEOUT) {
                        Ok(source) => {
                            let initial_version =
                                http_get_text(&dev_version_url(url)?, ANDROID_DEV_HTTP_TIMEOUT)
                                    .ok();
                            logger::info(format!("loaded Android Raster dev bundle: {url}"));
                            return Ok(LoadedAndroidBundle {
                                name: url.clone(),
                                source,
                                dev_urls,
                                initial_version,
                            });
                        }
                        Err(error) => {
                            if attempt == ANDROID_DEV_INITIAL_RETRY_COUNT {
                                logger::warn(format!(
                                    "failed to load Android dev bundle {url}: {error:#}"
                                ));
                            }
                        }
                    }
                }
                std::thread::sleep(ANDROID_DEV_INITIAL_RETRY_INTERVAL);
            }
            logger::warn("Android dev bundle unavailable; falling back to asset bundle");
        }
        Err(error) => {
            logger::info(format!("Android dev config unavailable: {error:#}"));
        }
    }

    let source = load_asset_string(app, ANDROID_BUNDLE_ASSET)?;
    logger::info(format!(
        "loaded Android Raster asset bundle: asset://{ANDROID_BUNDLE_ASSET}"
    ));
    Ok(LoadedAndroidBundle {
        name: format!("asset://{ANDROID_BUNDLE_ASSET}"),
        source,
        dev_urls: Vec::new(),
        initial_version: None,
    })
}

fn load_dev_urls(app: &AndroidApp) -> anyhow::Result<Vec<String>> {
    let source = load_asset_string(app, ANDROID_DEV_CONFIG_ASSET)?;
    let value: serde_json::Value = serde_json::from_str(&source)
        .map_err(|error| anyhow::anyhow!("invalid {ANDROID_DEV_CONFIG_ASSET}: {error}"))?;
    let urls = value
        .get("urls")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("{ANDROID_DEV_CONFIG_ASSET} must contain urls array"))?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if urls.is_empty() {
        anyhow::bail!("{ANDROID_DEV_CONFIG_ASSET} urls array is empty");
    }
    Ok(urls)
}

fn start_dev_server_reloader(
    dev_urls: Vec<String>,
    initial_version: Option<String>,
    sender: crate::common::channel::ChannelSender<RuntimeCommand>,
) {
    std::thread::spawn(move || {
        let mut last_version = initial_version;
        loop {
            std::thread::sleep(ANDROID_DEV_POLL_INTERVAL);
            let Some((bundle_url, version)) = fetch_first_dev_version(&dev_urls) else {
                continue;
            };
            if last_version.as_ref() == Some(&version) {
                continue;
            }

            match http_get_text(&bundle_url, ANDROID_DEV_HTTP_TIMEOUT) {
                Ok(source) => {
                    let command = RuntimeCommand::ReloadAppBundleSource {
                        name: bundle_url,
                        source,
                    };
                    if let Err(error) = RuntimeCommandQueue::enqueue(&sender, command) {
                        logger::warn(format!("Android dev reload stopped: {error:#}"));
                        break;
                    }
                    last_version = Some(version);
                }
                Err(error) => {
                    logger::warn(format!(
                        "failed to fetch Android dev bundle reload: {error:#}"
                    ));
                }
            }
        }
    });
}

fn fetch_first_dev_version(dev_urls: &[String]) -> Option<(String, String)> {
    for url in dev_urls {
        let version_url = match dev_version_url(url) {
            Ok(version_url) => version_url,
            Err(error) => {
                logger::warn(format!("invalid Android dev URL {url}: {error:#}"));
                continue;
            }
        };
        match http_get_text(&version_url, ANDROID_DEV_HTTP_TIMEOUT) {
            Ok(version) => return Some((url.clone(), version)),
            Err(error) => {
                logger::warn(format!(
                    "failed to poll Android dev bundle version {version_url}: {error:#}"
                ));
            }
        }
    }
    None
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

fn dev_version_url(bundle_url: &str) -> anyhow::Result<String> {
    let parsed = parse_http_url(bundle_url)?;
    Ok(format!("http://{}:{}/version", parsed.host, parsed.port))
}

fn http_get_text(url: &str, timeout: Duration) -> anyhow::Result<String> {
    let parsed = parse_http_url(url)?;
    let address = (parsed.host.as_str(), parsed.port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("failed to resolve {}", parsed.host))?;
    let mut stream = TcpStream::connect_timeout(&address, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\n\r\n",
        parsed.path, parsed.host, parsed.port
    );
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    parse_http_text_response(url, &response)
}

fn parse_http_text_response(url: &str, response: &[u8]) -> anyhow::Result<String> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| anyhow::anyhow!("invalid HTTP response from {url}"))?;
    let headers = std::str::from_utf8(&response[..header_end])
        .map_err(|error| anyhow::anyhow!("invalid HTTP headers from {url}: {error}"))?;
    let status_line = headers
        .lines()
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing HTTP status from {url}"))?;
    if !status_line.contains(" 200 ") {
        anyhow::bail!("HTTP request failed for {url}: {status_line}");
    }
    let body = &response[header_end + 4..];
    let body = if has_chunked_transfer_encoding(headers) {
        decode_chunked_body(body)?
    } else {
        body.to_vec()
    };
    String::from_utf8(body)
        .map_err(|error| anyhow::anyhow!("HTTP response from {url} is not UTF-8: {error}"))
}

fn has_chunked_transfer_encoding(headers: &str) -> bool {
    headers.lines().any(|line| {
        let Some((name, value)) = line.split_once(':') else {
            return false;
        };
        name.trim().eq_ignore_ascii_case("transfer-encoding")
            && value
                .split(',')
                .any(|item| item.trim().eq_ignore_ascii_case("chunked"))
    })
}

fn decode_chunked_body(mut body: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut decoded = Vec::new();
    loop {
        let Some(line_end) = find_crlf(body) else {
            anyhow::bail!("invalid chunked HTTP response: missing chunk size terminator");
        };
        let size_line = std::str::from_utf8(&body[..line_end])
            .map_err(|error| anyhow::anyhow!("invalid chunk size line: {error}"))?;
        let size_hex = size_line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_hex, 16)
            .map_err(|error| anyhow::anyhow!("invalid chunk size {size_hex:?}: {error}"))?;
        body = &body[line_end + 2..];
        if size == 0 {
            return Ok(decoded);
        }
        if body.len() < size + 2 {
            anyhow::bail!("invalid chunked HTTP response: chunk exceeds response length");
        }
        decoded.extend_from_slice(&body[..size]);
        if &body[size..size + 2] != b"\r\n" {
            anyhow::bail!("invalid chunked HTTP response: missing chunk terminator");
        }
        body = &body[size + 2..];
    }
}

fn find_crlf(bytes: &[u8]) -> Option<usize> {
    bytes.windows(2).position(|window| window == b"\r\n")
}

struct ParsedHttpUrl {
    host: String,
    port: u16,
    path: String,
}

fn parse_http_url(url: &str) -> anyhow::Result<ParsedHttpUrl> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| anyhow::anyhow!("only http:// URLs are supported: {url}"))?;
    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, format!("/{path}")),
        None => (rest, "/".to_owned()),
    };
    if authority.is_empty() {
        anyhow::bail!("HTTP URL host is empty: {url}");
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => {
            let port = port
                .parse::<u16>()
                .map_err(|error| anyhow::anyhow!("invalid HTTP URL port in {url}: {error}"))?;
            (host.to_owned(), port)
        }
        _ => (authority.to_owned(), 80),
    };
    Ok(ParsedHttpUrl { host, port, path })
}
