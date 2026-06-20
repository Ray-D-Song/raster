//! GPUI app thread side of Raster.
//!
//! This side owns windows, GPUI entities, the NativeObjectTree, RenderModels,
//! mounting diff application, and owner notifications.

pub mod app;
pub mod components;
pub mod config_provider;
pub mod notification;
pub mod perf;
pub mod render_model;
pub mod retained_tree;
pub mod theme_snapshot;

pub use app::{DevReloadConfig, open_raster_window};

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use app::start_desktop;
