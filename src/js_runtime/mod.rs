//! JS worker side of Raster.
//!
//! This side owns the JS runtime and the React host functions that forward
//! mutation-mode host calls to the GPUI app thread. It must not mutate GPUI
//! entities or GPUI-facing retained objects directly.

pub mod bundle;
pub mod host;
pub mod module_loader;
pub mod vm;

pub use vm::start;
