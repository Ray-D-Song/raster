use std::{
    ffi::{CStr, CString, c_char, c_void},
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::{Mutex, OnceLock},
    time::Duration,
};

use gpui::{App, WindowOptions};

use crate::{
    app::{RasterBundle, RasterRunOptions, prepare_raster_app},
    common::{
        channel::{RuntimeCommand, RuntimeCommandQueue},
        utils::logger::{self, LogLevel, LoggerConfig},
    },
    config::{DEFAULT_ROOT_HEIGHT, DEFAULT_ROOT_WIDTH},
    gpui_backend,
};

const IOS_DEV_HTTP_TIMEOUT: Duration = Duration::from_millis(800);
const IOS_DEV_INITIAL_RETRY_COUNT: usize = 25;
const IOS_DEV_INITIAL_RETRY_INTERVAL: Duration = Duration::from_millis(200);
const IOS_DEV_SSE_RECONNECT_INTERVAL: Duration = Duration::from_millis(500);
const IOS_DEV_SSE_READ_TIMEOUT: Duration = Duration::from_secs(30);

static LAST_ERROR: OnceLock<Mutex<Option<CString>>> = OnceLock::new();

#[derive(Debug)]
struct IosDevConfig {
    urls: Vec<String>,
}

struct LoadedIosBundle {
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
pub unsafe extern "C" fn raster_ios_run_app(
    bundle_name: *const c_char,
    bundle_source: *const c_char,
    dev_config_json: *const c_char,
) -> bool {
    clear_last_error();
    match unsafe { run_app(bundle_name, bundle_source, dev_config_json) } {
        Ok(()) => true,
        Err(error) => {
            set_last_error(format!("{error:#}"));
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn raster_ios_last_error() -> *const c_char {
    let guard = LAST_ERROR
        .get_or_init(|| Mutex::new(None))
        .lock()
        .expect("last error lock");
    guard
        .as_ref()
        .map(|message| message.as_ptr())
        .unwrap_or(std::ptr::null())
}

#[unsafe(no_mangle)]
pub extern "C" fn raster_ios_request_frame() {
    let window = gpui_mobile::ios::ffi::gpui_ios_get_window();
    if !window.is_null() {
        gpui_mobile::ios::ffi::gpui_ios_request_frame(window);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn raster_ios_will_enter_foreground() {
    with_ios_window(|window| {
        gpui_mobile::ios::ffi::gpui_ios_will_enter_foreground(window);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn raster_ios_did_become_active() {
    with_ios_window(|window| {
        gpui_mobile::ios::ffi::gpui_ios_did_become_active(window);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn raster_ios_will_resign_active() {
    with_ios_window(|window| {
        gpui_mobile::ios::ffi::gpui_ios_will_resign_active(window);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn raster_ios_did_enter_background() {
    with_ios_window(|window| {
        gpui_mobile::ios::ffi::gpui_ios_did_enter_background(window);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn raster_ios_will_terminate() {
    with_ios_window(|window| {
        gpui_mobile::ios::ffi::gpui_ios_will_terminate(window);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn raster_ios_handle_open_url(url_ptr: *mut c_void) {
    gpui_mobile::ios::ffi::gpui_ios_handle_open_url(url_ptr);
}

fn with_ios_window(callback: impl FnOnce(*mut c_void)) {
    let window = gpui_mobile::ios::ffi::gpui_ios_get_window();
    if !window.is_null() {
        callback(window);
    }
}

unsafe fn run_app(
    bundle_name: *const c_char,
    bundle_source: *const c_char,
    dev_config_json: *const c_char,
) -> anyhow::Result<()> {
    if bundle_name.is_null() {
        anyhow::bail!("bundle_name must not be null");
    }

    let _ = logger::init(LoggerConfig {
        level: LogLevel::Info,
        file_path: None,
    });

    let name = unsafe { CStr::from_ptr(bundle_name) }
        .to_str()
        .map_err(|error| anyhow::anyhow!("bundle_name is not valid UTF-8: {error}"))?
        .to_owned();
    let dev_config = if dev_config_json.is_null() {
        None
    } else {
        Some(parse_dev_config(
            unsafe { CStr::from_ptr(dev_config_json) }.to_str()?,
        )?)
    };
    let loaded_bundle = if let Some(config) = dev_config {
        load_dev_bundle(config.urls)?
    } else {
        if bundle_source.is_null() {
            anyhow::bail!("bundle_source must not be null in production mode");
        }
        let source = unsafe { CStr::from_ptr(bundle_source) }
            .to_str()
            .map_err(|error| anyhow::anyhow!("bundle_source is not valid UTF-8: {error}"))?
            .to_owned();
        LoadedIosBundle {
            name,
            source,
            dev_urls: Vec::new(),
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
    let prepared = pollster::block_on(prepare_raster_app(&options))?;
    if dev_mode {
        start_dev_server_reloader(loaded_bundle.dev_urls, prepared.runtime_commands.clone());
    }

    gpui_mobile::ios::ffi::set_app_callback(Box::new(move |cx: &mut App| {
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
    }));
    gpui_mobile::ios::ffi::run_app();
    Ok(())
}

fn parse_dev_config(source: &str) -> anyhow::Result<IosDevConfig> {
    let value: serde_json::Value = serde_json::from_str(source)
        .map_err(|error| anyhow::anyhow!("invalid iOS dev config: {error}"))?;
    let urls = value
        .get("urls")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|url| !url.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .ok_or_else(|| anyhow::anyhow!("iOS dev config must contain urls array"))?;
    if urls.is_empty() {
        anyhow::bail!("iOS dev config urls array is empty");
    }
    Ok(IosDevConfig { urls })
}

fn load_dev_bundle(dev_urls: Vec<String>) -> anyhow::Result<LoadedIosBundle> {
    for attempt in 0..=IOS_DEV_INITIAL_RETRY_COUNT {
        for url in &dev_urls {
            match http_get_text(url, IOS_DEV_HTTP_TIMEOUT) {
                Ok(source) => {
                    logger::info(format!("loaded iOS Raster dev bundle: {url}"));
                    return Ok(LoadedIosBundle {
                        name: url.clone(),
                        source,
                        dev_urls,
                    });
                }
                Err(error) => {
                    if attempt == IOS_DEV_INITIAL_RETRY_COUNT {
                        logger::warn(format!("failed to load iOS dev bundle {url}: {error:#}"));
                    }
                }
            }
        }
        std::thread::sleep(IOS_DEV_INITIAL_RETRY_INTERVAL);
    }
    anyhow::bail!("iOS dev bundle unavailable")
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
                        logger::warn("iOS dev event stream closed");
                    }
                    Err(error) => {
                        logger::warn(format!("iOS dev event stream failed: {error:#}"));
                    }
                }
                std::thread::sleep(IOS_DEV_SSE_RECONNECT_INTERVAL);
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
    let mut http_stream = http_get_stream(&events_url, IOS_DEV_HTTP_TIMEOUT)?;
    let mut stream = http_stream.stream;
    stream.set_read_timeout(Some(IOS_DEV_SSE_READ_TIMEOUT))?;

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
            match http_get_text(&reload_url, IOS_DEV_HTTP_TIMEOUT) {
                Ok(source) => {
                    let command = RuntimeCommand::ReloadAppBundleSource {
                        name: reload_url,
                        source,
                    };
                    if RuntimeCommandQueue::enqueue(sender, command).is_err() {
                        return Ok(());
                    }
                    *last_version = Some(event.version);
                }
                Err(error) => {
                    logger::warn(format!("failed to fetch iOS dev bundle reload: {error:#}"));
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
        .map_err(|error| anyhow::anyhow!("invalid iOS bundle event: {error}"))?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("iOS bundle event missing version"))?
        .to_owned();
    let url = value
        .get("url")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("iOS bundle event missing url"))?
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

fn clear_last_error() {
    *LAST_ERROR
        .get_or_init(|| Mutex::new(None))
        .lock()
        .expect("last error lock") = None;
}

fn set_last_error(message: String) {
    let sanitized = message.replace('\0', "\\0");
    *LAST_ERROR
        .get_or_init(|| Mutex::new(None))
        .lock()
        .expect("last error lock") =
        Some(CString::new(sanitized).expect("sanitized error message"));
}
