// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
fn main() {
    if std::env::var("CARGO_FEATURE_NAPI").is_ok() {
        #[cfg(all(unix, not(target_os = "macos")))]
        println!("cargo:rustc-link-arg=-rdynamic");

        #[cfg(target_os = "macos")]
        println!("cargo:rustc-link-arg=-Wl,-export_dynamic");
    }
}
