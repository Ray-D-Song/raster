//! Per-runtime isolate and per-context state handles (opaque to Rust).

#[repr(C)]
pub struct IsolateState {
    _private: [u8; 0],
}

#[repr(C)]
pub struct ContextState {
    _private: [u8; 0],
}
