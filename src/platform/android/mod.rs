use std::{
    ffi::CString,
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

use android_activity::AndroidApp;
use gpui::{App, Application, WindowOptions};
use gpui_mobile::android::jni as gpui_jni;
use jni::{JavaVM, objects::JObject};

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
const ANDROID_DEV_HTTP_TIMEOUT: Duration = Duration::from_millis(800);
const ANDROID_DEV_SSE_RECONNECT_INTERVAL: Duration = Duration::from_millis(500);
const ANDROID_DEV_SSE_READ_TIMEOUT: Duration = Duration::from_secs(30);

struct LoadedAndroidBundle {
    name: String,
    source: String,
    dev_urls: Vec<String>,
}

#[derive(Debug)]
struct BundleEvent {
    version: String,
    url: String,
}

struct HttpStream {
    stream: TcpStream,
    initial_body: String,
}

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("raster"),
    );
    gpui_jni::install_panic_hook();

    let _ = logger::init(LoggerConfig {
        level: LogLevel::Info,
        file_path: None,
    });
    logger::info("android_main entered");

    let debuggable = match is_app_debuggable(&app) {
        Ok(debuggable) => debuggable,
        Err(error) => {
            logger::error(format!("failed to read Android debuggable flag: {error:#}"));
            return;
        }
    };
    let loaded_bundle = match load_android_bundle(&app, debuggable) {
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
        start_dev_server_reloader(loaded_bundle.dev_urls, prepared.runtime_commands.clone());
    }

    let _platform = gpui_jni::init_platform(&app);
    let Some(shared_platform) = gpui_jni::shared_platform() else {
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

fn load_android_bundle(app: &AndroidApp, debuggable: bool) -> anyhow::Result<LoadedAndroidBundle> {
    if debuggable {
        let dev_urls = load_dev_urls(app)?;
        for attempt in 0..=ANDROID_DEV_INITIAL_RETRY_COUNT {
            for url in &dev_urls {
                match http_get_text(url, ANDROID_DEV_HTTP_TIMEOUT) {
                    Ok(source) => {
                        logger::info(format!("loaded Android Raster dev bundle: {url}"));
                        return Ok(LoadedAndroidBundle {
                            name: url.clone(),
                            source,
                            dev_urls,
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
        anyhow::bail!("Android dev bundle unavailable");
    }

    let source = load_asset_string(app, ANDROID_BUNDLE_ASSET)?;
    logger::info(format!(
        "loaded Android Raster asset bundle: asset://{ANDROID_BUNDLE_ASSET}"
    ));
    Ok(LoadedAndroidBundle {
        name: format!("asset://{ANDROID_BUNDLE_ASSET}"),
        source,
        dev_urls: Vec::new(),
    })
}

fn is_app_debuggable(app: &AndroidApp) -> anyhow::Result<bool> {
    const FLAG_DEBUGGABLE: i32 = 0x2;
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    let mut env = vm.attach_current_thread()?;
    let activity = unsafe { JObject::from_raw(&env, app.activity_as_ptr().cast()) };
    let application_info = env
        .call_method(
            &activity,
            "getApplicationInfo",
            "()Landroid/content/pm/ApplicationInfo;",
            &[],
        )?
        .l()?;
    let flags = env.get_field(&application_info, "flags", "I")?.i()?;
    Ok((flags & FLAG_DEBUGGABLE) != 0)
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
    sender: crate::common::channel::ChannelSender<RuntimeCommand>,
) {
    std::thread::spawn(move || {
        let mut last_version = None;
        loop {
            for dev_url in &dev_urls {
                match listen_for_bundle_events(dev_url, &mut last_version, &sender) {
                    Ok(()) => {
                        logger::warn("Android dev event stream closed");
                    }
                    Err(error) => {
                        logger::warn(format!("Android dev event stream failed: {error:#}"));
                    }
                }
                std::thread::sleep(ANDROID_DEV_SSE_RECONNECT_INTERVAL);
            }
        }
    });
}

fn listen_for_bundle_events(
    bundle_url: &str,
    last_version: &mut Option<String>,
    sender: &crate::common::channel::ChannelSender<RuntimeCommand>,
) -> anyhow::Result<()> {
    let events_url = dev_events_url(bundle_url)?;
    let mut http_stream = http_get_stream(&events_url, ANDROID_DEV_HTTP_TIMEOUT)?;
    let mut stream = http_stream.stream;
    stream.set_read_timeout(Some(ANDROID_DEV_SSE_READ_TIMEOUT))?;

    let mut pending = std::mem::take(&mut http_stream.initial_body);
    let mut read_buf = [0_u8; 4096];
    loop {
        while let Some(raw_event) = take_sse_event(&mut pending) {
            let Some(event) = parse_bundle_event(&raw_event)? else {
                continue;
            };
            if last_version.as_ref() == Some(&event.version) {
                continue;
            }

            let reload_url = resolve_dev_url(bundle_url, &event.url)?;
            match http_get_text(&reload_url, ANDROID_DEV_HTTP_TIMEOUT) {
                Ok(source) => {
                    let command = RuntimeCommand::ReloadAppBundleSource {
                        name: reload_url,
                        source,
                    };
                    if let Err(error) = RuntimeCommandQueue::enqueue(sender, command) {
                        logger::warn(format!("Android dev reload stopped: {error:#}"));
                        return Ok(());
                    }
                    *last_version = Some(event.version);
                }
                Err(error) => {
                    logger::warn(format!(
                        "failed to fetch Android dev bundle reload: {error:#}"
                    ));
                }
            }
        }

        let read = stream.read(&mut read_buf)?;
        if read == 0 {
            return Ok(());
        }
        pending.push_str(std::str::from_utf8(&read_buf[..read])?);
    }
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

fn dev_events_url(bundle_url: &str) -> anyhow::Result<String> {
    let parsed = parse_http_url(bundle_url)?;
    Ok(format!("http://{}:{}/events", parsed.host, parsed.port))
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

fn http_get_stream(url: &str, timeout: Duration) -> anyhow::Result<HttpStream> {
    let parsed = parse_http_url(url)?;
    let address = (parsed.host.as_str(), parsed.port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("failed to resolve {}", parsed.host))?;
    let mut stream = TcpStream::connect_timeout(&address, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}:{}\r\nAccept: text/event-stream\r\nConnection: keep-alive\r\n\r\n",
        parsed.path, parsed.host, parsed.port
    );
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            anyhow::bail!("HTTP stream closed before headers from {url}");
        }
        response.extend_from_slice(&buffer[..read]);
        if let Some(header_end) = response.windows(4).position(|window| window == b"\r\n\r\n") {
            let headers = std::str::from_utf8(&response[..header_end])
                .map_err(|error| anyhow::anyhow!("invalid HTTP headers from {url}: {error}"))?;
            let status_line = headers
                .lines()
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing HTTP status from {url}"))?;
            if !status_line.contains(" 200 ") {
                anyhow::bail!("HTTP request failed for {url}: {status_line}");
            }
            let initial_body = std::str::from_utf8(&response[header_end + 4..])
                .map_err(|error| anyhow::anyhow!("invalid HTTP stream body from {url}: {error}"))?
                .to_owned();
            return Ok(HttpStream {
                stream,
                initial_body,
            });
        }
    }
}

fn take_sse_event(pending: &mut String) -> Option<String> {
    let normalized = pending.replace("\r\n", "\n");
    let event_end = normalized.find("\n\n")?;
    let event = normalized[..event_end].to_owned();
    *pending = normalized[event_end + 2..].to_owned();
    Some(event)
}

fn parse_bundle_event(raw_event: &str) -> anyhow::Result<Option<BundleEvent>> {
    let mut event_type = "message";
    let mut data = String::new();
    for line in raw_event.lines() {
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        if let Some(value) = line.strip_prefix("event:") {
            event_type = value.trim();
        } else if let Some(value) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(value.trim_start());
        }
    }
    if event_type != "bundle" || data.is_empty() {
        return Ok(None);
    }
    let value: serde_json::Value = serde_json::from_str(&data)
        .map_err(|error| anyhow::anyhow!("invalid Android bundle event: {error}"))?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Android bundle event missing version"))?
        .to_owned();
    let url = value
        .get("url")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Android bundle event missing url"))?
        .to_owned();
    Ok(Some(BundleEvent { version, url }))
}

fn resolve_dev_url(base_url: &str, event_url: &str) -> anyhow::Result<String> {
    if event_url.starts_with("http://") {
        return Ok(event_url.to_owned());
    }
    let parsed = parse_http_url(base_url)?;
    if event_url.starts_with('/') {
        return Ok(format!(
            "http://{}:{}{}",
            parsed.host, parsed.port, event_url
        ));
    }
    let directory = parsed
        .path
        .rsplit_once('/')
        .map(
            |(directory, _)| {
                if directory.is_empty() { "/" } else { directory }
            },
        )
        .unwrap_or("/");
    let path = if directory == "/" {
        format!("/{event_url}")
    } else {
        format!("{}/{}", directory.trim_end_matches('/'), event_url)
    };
    Ok(format!("http://{}:{}{}", parsed.host, parsed.port, path))
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
