// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::env;

fn main() {
    let v8_compat = env::var("CARGO_FEATURE_V8_COMPAT").is_ok();
    let napi = env::var("CARGO_FEATURE_NAPI").is_ok();
    if napi || v8_compat {
        // Export dynamic symbols so native .node addons can resolve V8 ABI
        // symbols from the host executable. The V8 shim archive itself is
        // linked solely by v8_compat's build.rs (static:+whole-archive).
        #[cfg(all(unix, not(target_os = "macos")))]
        println!("cargo:rustc-link-arg=-rdynamic");

        #[cfg(target_os = "macos")]
        println!("cargo:rustc-link-arg=-Wl,-export_dynamic");
    }
}
