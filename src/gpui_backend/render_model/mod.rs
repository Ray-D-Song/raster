//! Parsed GPUI-facing data models.
//!
//! `retained_tree` owns node lifetime and calls this module when payloads
//! change. Component adapters later project these models into GPUI elements.

pub mod build;
pub mod model;
pub mod style;
