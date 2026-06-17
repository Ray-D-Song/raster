use std::sync::OnceLock;

pub const DEFAULT_ROOT_WIDTH: u32 = 800;
pub const DEFAULT_ROOT_HEIGHT: u32 = 600;
#[cfg(test)]
pub const DEFAULT_ROOT_SIZE: (u32, u32) = (DEFAULT_ROOT_WIDTH, DEFAULT_ROOT_HEIGHT);
pub const APP_BUNDLE_PATH: &str = "target/raster/app.js";
// React's commit traversal is recursive; the LLRT default 512 KiB stack is too small for app trees.
pub const JS_MAX_STACK_SIZE: usize = 8 * 1024 * 1024;

const ROOT_PACKAGE_JSON: &str = include_str!("../../package.json");
static VERSION: OnceLock<String> = OnceLock::new();

pub fn version() -> &'static str {
    VERSION
        .get_or_init(|| {
            let package: serde_json::Value = serde_json::from_str(ROOT_PACKAGE_JSON)
                .expect("root package.json must be valid JSON");
            package
                .get("version")
                .and_then(serde_json::Value::as_str)
                .filter(|version| !version.is_empty())
                .expect("root package.json must define a non-empty string version")
                .to_owned()
        })
        .as_str()
}
