// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::env;

use once_cell::sync::Lazy;

pub mod module;
pub mod module_builder;
pub mod package;

pub use self::modules::*;

mod modules {
    #[cfg(feature = "abort")]
    pub use raster_runtime_abort as abort;
    #[cfg(feature = "assert")]
    pub use raster_runtime_assert as assert;
    #[cfg(feature = "async-hooks")]
    pub use raster_runtime_async_hooks as async_hooks;
    #[cfg(feature = "buffer")]
    pub use raster_runtime_buffer as buffer;
    #[cfg(feature = "child-process")]
    pub use raster_runtime_child_process as child_process;
    #[cfg(feature = "console")]
    pub use raster_runtime_console as console;
    #[cfg(feature = "constants")]
    pub use raster_runtime_constants as constants;
    #[cfg(feature = "crypto")]
    pub use raster_runtime_crypto as crypto;
    #[cfg(feature = "dgram")]
    pub use raster_runtime_dgram as dgram;
    #[cfg(feature = "diagnostics-channel")]
    pub use raster_runtime_diagnostics_channel as diagnostics_channel;
    #[cfg(feature = "dns")]
    pub use raster_runtime_dns as dns;
    #[cfg(feature = "events")]
    pub use raster_runtime_events as events;
    #[cfg(feature = "exceptions")]
    pub use raster_runtime_exceptions as exceptions;
    #[cfg(feature = "fetch")]
    pub use raster_runtime_fetch as fetch;
    #[cfg(feature = "fs")]
    pub use raster_runtime_fs as fs;
    #[cfg(feature = "http")]
    pub use raster_runtime_http as http;
    #[cfg(feature = "https")]
    pub use raster_runtime_http as https;
    #[cfg(feature = "intl")]
    pub use raster_runtime_intl as intl;
    #[cfg(feature = "navigator")]
    pub use raster_runtime_navigator as navigator;
    #[cfg(feature = "net")]
    pub use raster_runtime_net as net;
    #[cfg(feature = "os")]
    pub use raster_runtime_os as os;
    #[cfg(feature = "path")]
    pub use raster_runtime_path as path;
    #[cfg(feature = "perf-hooks")]
    pub use raster_runtime_perf_hooks as perf_hooks;
    #[cfg(feature = "process")]
    pub use raster_runtime_process as process;
    #[cfg(feature = "querystring")]
    pub use raster_runtime_querystring as querystring;
    #[cfg(feature = "stream-web")]
    pub use raster_runtime_stream_web as stream_web;
    #[cfg(feature = "string-decoder")]
    pub use raster_runtime_string_decoder as string_decoder;
    #[cfg(feature = "temporal")]
    pub use raster_runtime_temporal as temporal;
    #[cfg(feature = "timers")]
    pub use raster_runtime_timers as timers;
    #[cfg(feature = "tls")]
    pub use raster_runtime_tls as tls;
    #[cfg(feature = "tty")]
    pub use raster_runtime_tty as tty;
    #[cfg(feature = "url")]
    pub use raster_runtime_url as url;
    #[cfg(feature = "util")]
    pub use raster_runtime_util as util;
    #[cfg(feature = "v8")]
    pub use raster_runtime_v8 as v8;
    #[cfg(feature = "vm")]
    pub use raster_runtime_vm as vm;
    #[cfg(feature = "webassembly")]
    pub use raster_runtime_webassembly as webassembly;
    #[cfg(feature = "zlib")]
    pub use raster_runtime_zlib as zlib;
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// added when .cjs files are imported
pub const CJS_IMPORT_PREFIX: &str = "__cjs:";
// added to force CJS imports in loader
pub const CJS_LOADER_PREFIX: &str = "__cjsm:";

pub const ENV_RASTER_RUNTIME_PLATFORM: &str = "RASTER_RUNTIME_PLATFORM";

pub static RASTER_RUNTIME_PLATFORM: Lazy<String> = Lazy::new(|| {
    env::var(ENV_RASTER_RUNTIME_PLATFORM)
        .ok()
        .filter(|platform| platform == "node")
        .unwrap_or_else(|| "browser".to_string())
});
