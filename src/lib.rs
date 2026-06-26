pub mod app;
pub mod bridge;
pub mod common;
pub mod config;
pub mod gpui_backend;
pub mod js_runtime;
pub mod plugin;

#[cfg(target_os = "android")]
#[path = "platform/android/mod.rs"]
pub mod android;

#[cfg(target_os = "ios")]
#[path = "platform/ios/mod.rs"]
pub mod ios;
