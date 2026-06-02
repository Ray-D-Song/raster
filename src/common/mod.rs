//! Shared data types used by both the JS worker side and the GPUI app thread.
//!
//! This module should stay free of GPUI and JS engine dependencies. It is for
//! ids, payload schemas, style schemas, event metadata, and other plain data
//! that can safely cross thread boundaries.

pub mod channel;
pub mod ids;
pub mod mount;
pub mod utils;
