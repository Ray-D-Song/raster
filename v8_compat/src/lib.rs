//! QuickJS-backed V8/Node C++ ABI shim (Node 24 / ABI 137).

#![allow(dead_code)] // FFI surface includes stubs not yet wired by all addons.

pub mod abi;
#[cfg(test)]
mod abi_tests;
mod bridge;
mod context_tables;
mod exports;
mod isolate;
mod js_ops;
mod module_loader;
mod owned_js_value;
mod root;
mod runtime_state;
mod value_ops;

pub use abi::*;
pub use bridge::{
    bind_bridge, ensure_shim_linked, napi_api_version, napi_node_version_u32, node_abi_version,
    prepare_shutdown, teardown_counts_for_ctx, TeardownCounts,
};
pub use isolate::{ContextState, IsolateState};
pub use module_loader::{
    clear_pending_v8_modules, contexts_for_runtime, drain_pending_v8_modules,
    drain_pending_v8_modules_since, ensure_context_for_ctx, ensure_isolate_for_runtime,
    isolate_ptr_for_runtime, materialize_exports, push_native_load, run_pre_bridge_teardown_gc,
    run_v8_module_init, shutdown_context, shutdown_environment, shutdown_runtime,
    shutdown_runtime_addr, NativeLoadFrame, NativeLoadGuard, NodeModule,
};
