# Vendored dependencies

This directory contains source snapshots that Raster patches directly.

## zed

- Upstream: https://github.com/zed-industries/zed
- Commit: `5688167d224b5eca54875d49afb8bfd73a07915a`
- Used for: `gpui`, `gpui_platform`, `gpui_wgpu`, `gpui_web`, `gpui_macros`, `reqwest_client`, and related GPUI workspace crates.
- Update process: replace `deps/zed` with a clean upstream snapshot, preserve local Raster patches, then regenerate `Cargo.lock` and run the Rust validation commands.

## gpui-mobile

- Upstream: https://github.com/Ray-D-Song/raster-gpui-mobile
- Used for: Android and iOS GPUI platform integration.
- Update process: apply upstream changes into this directory as normal source changes. Do not restore nested `.git` metadata.

## gpui-component

- Upstream: https://github.com/longbridge/gpui-component
- Used for: Raster's GPUI component rendering layer.
- Update process: apply upstream changes into this directory as normal source changes. Do not restore nested `.git` metadata.

## raster-runtime

- Upstream: Raster-maintained runtime source dependency.
- Used for: embedded JavaScript runtime crates.
