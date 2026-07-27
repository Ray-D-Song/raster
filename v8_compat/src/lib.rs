//! QuickJS-backed V8/Node C++ ABI shim (Node 24 / ABI 137).

pub mod abi;
#[cfg(test)]
mod abi_tests;
mod bridge;
mod exports;
mod isolate;
mod js_ops;
mod module_loader;
mod root;
mod value_ops;

pub use abi::*;
pub use bridge::{bind_bridge, ensure_shim_linked, napi_api_version, napi_node_version_u32, node_abi_version, prepare_shutdown};
pub use isolate::{ContextState, IsolateState};
pub use module_loader::{
    clear_pending_v8_modules, drain_pending_v8_modules, ensure_context_for_ctx,
    ensure_isolate_for_runtime, push_native_load, run_v8_module_init, shutdown_runtime,
    NativeLoadFrame, NativeLoadGuard, NodeModule,
};
