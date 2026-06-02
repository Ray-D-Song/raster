//! Retained UI objects owned by the GPUI app thread.
//!
//! This module is the backend-side equivalent of a small DOM. React/JS will
//! eventually send ordered mutations, and the GPUI app thread will apply them
//! here before rendering temporary GPUI elements from the retained models.

#![allow(dead_code)]

pub mod diff;
pub mod mutation;
pub mod node;
pub mod tree;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
