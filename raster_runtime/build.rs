// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::env;

fn main() {
    let v8_compat = env::var("CARGO_FEATURE_V8_COMPAT").is_ok();
    let napi = env::var("CARGO_FEATURE_NAPI").is_ok();
    if napi || v8_compat {
        #[cfg(all(unix, not(target_os = "macos")))]
        println!("cargo:rustc-link-arg=-rdynamic");

        #[cfg(target_os = "macos")]
        println!("cargo:rustc-link-arg=-Wl,-export_dynamic");
    }

    if v8_compat {
        link_v8_shim_for_host();
    }
}

fn link_v8_shim_for_host() {
    let out_dir = match env::var("DEP_RASTER_V8_OUT_DIR") {
        Ok(path) => path,
        Err(_) => return,
    };
    let archive = format!("{out_dir}/libraster_v8_shim.a");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-link-arg=-Wl,-force_load,{archive}");
            println!("cargo:rustc-link-lib=c++");
        }
        "linux" => {
            println!("cargo:rustc-link-search=native={out_dir}");
            println!("cargo:rustc-link-arg=-Wl,--whole-archive");
            println!("cargo:rustc-link-lib=static=raster_v8_shim");
            println!("cargo:rustc-link-arg=-Wl,--no-whole-archive");
            println!("cargo:rustc-link-lib=c++");
        }
        _ => {}
    }
}
