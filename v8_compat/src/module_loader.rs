use std::cell::RefCell;
use std::os::raw::c_char;
use std::ptr;
use std::sync::OnceLock;

use rquickjs::qjs::{self, JSContext, JSValue};

use crate::abi::NODE_MODULE_VERSION;
use crate::bridge::with_bridge_roots;
use crate::isolate::{ContextState, IsolateState};

thread_local! {
    static NATIVE_LOAD_STACK: RefCell<Vec<NativeLoadFrame>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Copy)]
pub struct NativeLoadFrame {
    pub ctx: *mut JSContext,
    pub isolate: *mut IsolateState,
    pub context_state: *mut ContextState,
}

pub struct NativeLoadGuard {
    frame: NativeLoadFrame,
}

impl NativeLoadGuard {
    pub fn enter(ctx: *mut JSContext, isolate: *mut IsolateState, context_state: *mut ContextState) -> Self {
        let frame = NativeLoadFrame {
            ctx,
            isolate,
            context_state,
        };
        NATIVE_LOAD_STACK.with(|stack| stack.borrow_mut().push(frame));
        unsafe {
            raster_v8_set_current_context(context_state as *mut _);
            raster_v8_set_current_isolate(isolate as *mut _);
        }
        Self { frame }
    }
}

impl Drop for NativeLoadGuard {
    fn drop(&mut self) {
        NATIVE_LOAD_STACK.with(|stack| {
            let mut s = stack.borrow_mut();
            s.pop();
            if let Some(prev) = s.last() {
                unsafe {
                    raster_v8_set_current_context(prev.context_state as *mut _);
                    raster_v8_set_current_isolate(prev.isolate as *mut _);
                }
            } else {
                unsafe {
                    raster_v8_set_current_context(ptr::null_mut());
                    raster_v8_set_current_isolate(ptr::null_mut());
                }
            }
        });
    }
}

pub fn push_native_load(ctx: *mut JSContext, isolate: *mut IsolateState, context_state: *mut ContextState) -> NativeLoadGuard {
    NativeLoadGuard::enter(ctx, isolate, context_state)
}

#[repr(C)]
pub struct NodeModule {
    pub nm_version: i32,
    pub nm_flags: u32,
    pub nm_dso_handle: *mut std::ffi::c_void,
    pub nm_filename: *const c_char,
    pub nm_register_func: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, *mut std::ffi::c_void)>,
    pub nm_context_register_func: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, *mut std::ffi::c_void, *mut std::ffi::c_void)>,
    pub nm_modname: *const c_char,
    pub nm_priv: *mut std::ffi::c_void,
    pub nm_link: *mut NodeModule,
}

pub struct V8ModuleRecord {
    pub module: *mut NodeModule,
}

extern "C" {
    fn raster_v8_set_current_context(ctx: *mut ContextState);
    fn raster_v8_set_current_isolate(isolate: *mut IsolateState);
    fn raster_v8_create_isolate() -> *mut IsolateState;
    fn raster_v8_destroy_isolate(isolate: *mut IsolateState);
    fn raster_v8_create_context() -> *mut ContextState;
    fn raster_v8_destroy_context(ctx: *mut ContextState);
    fn raster_v8_open_handle_scope(ctx: *mut ContextState);
    fn raster_v8_close_handle_scope(ctx: *mut ContextState);
    fn raster_v8_set_context_root_id(ctx: *mut ContextState, root_id: u64);
    fn raster_v8_run_module_init(
        ctx: *mut ContextState,
        module: *mut NodeModule,
        exports_root_id: u64,
        module_root_id: u64,
        out_exports_root_id: *mut u64,
    ) -> i32;
}

pub fn ensure_isolate_for_runtime(rt: *mut qjs::JSRuntime) -> *mut IsolateState {
    static ISOLATES: OnceLock<parking_lot::Mutex<std::collections::HashMap<usize, usize>>> =
        OnceLock::new();
    let map = ISOLATES.get_or_init(|| parking_lot::Mutex::new(std::collections::HashMap::new()));
    let key = rt as usize;
    let mut guard = map.lock();
    if let Some(isolate) = guard.get(&key) {
        return *isolate as *mut IsolateState;
    }
    let isolate = unsafe { raster_v8_create_isolate() };
    guard.insert(key, isolate as usize);
    isolate
}

pub fn ensure_context_for_ctx(ctx: *mut JSContext) -> *mut ContextState {
    static CONTEXTS: OnceLock<parking_lot::Mutex<std::collections::HashMap<usize, usize>>> =
        OnceLock::new();
    let map = CONTEXTS.get_or_init(|| parking_lot::Mutex::new(std::collections::HashMap::new()));
    let key = ctx as usize;
    let mut guard = map.lock();
    if let Some(state) = guard.get(&key) {
        return *state as *mut ContextState;
    }
    let state = unsafe { raster_v8_create_context() };
    let context_root = with_bridge_roots(|ctx, roots| {
        let obj = unsafe { qjs::JS_NewObject(ctx) };
        let root = roots.insert(ctx, obj);
        unsafe { raster_v8_set_context_root_id(state, root) };
        root
    });
    let _ = context_root;
    guard.insert(key, state as usize);
    state
}

pub fn shutdown_runtime(rt: *mut qjs::JSRuntime) {
    static ISOLATES: OnceLock<parking_lot::Mutex<std::collections::HashMap<usize, usize>>> =
        OnceLock::new();
    static CONTEXTS: OnceLock<parking_lot::Mutex<std::collections::HashMap<usize, usize>>> =
        OnceLock::new();

    let rt_key = rt as usize;
    if let Some(isolate) = ISOLATES.get_or_init(|| parking_lot::Mutex::new(std::collections::HashMap::new())).lock().remove(&rt_key) {
        unsafe {
            raster_v8_destroy_isolate(isolate as *mut IsolateState);
        }
    }

    let contexts = CONTEXTS.get_or_init(|| parking_lot::Mutex::new(std::collections::HashMap::new()));
    let mut guard = contexts.lock();
    let stale: Vec<usize> = guard
        .keys()
        .copied()
        .filter(|ctx_key| {
            let ctx = *ctx_key as *mut qjs::JSContext;
            unsafe { qjs::JS_GetRuntime(ctx) == rt }
        })
        .collect();
    for ctx_key in stale {
        if let Some(state) = guard.remove(&ctx_key) {
            unsafe {
                raster_v8_destroy_context(state as *mut ContextState);
            }
        }
    }
}

pub fn drain_pending_v8_modules() -> Vec<*mut NodeModule> {
    extern "C" {
        fn raster_v8_pending_modules_count() -> usize;
        fn raster_v8_take_pending_module(index: usize) -> *mut NodeModule;
    }
    let count = unsafe { raster_v8_pending_modules_count() };
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let module = unsafe { raster_v8_take_pending_module(i) };
        if !module.is_null() {
            out.push(module);
        }
    }
    out
}

pub fn clear_pending_v8_modules() {
    let _ = drain_pending_v8_modules();
}

pub unsafe fn run_v8_module_init(
    ctx: *mut JSContext,
    module: *mut NodeModule,
    exports: JSValue,
) -> Result<(), String> {
    if (*module).nm_version != NODE_MODULE_VERSION {
        return Err(format!(
            "unsupported NODE_MODULE_VERSION {} (expected {})",
            (*module).nm_version,
            NODE_MODULE_VERSION
        ));
    }
    let rt = qjs::JS_GetRuntime(ctx);
    let isolate = ensure_isolate_for_runtime(rt);
    let context_state = ensure_context_for_ctx(ctx);
    let _guard = push_native_load(ctx, isolate, context_state);

    let exports_root = with_bridge_roots(|ctx, roots| roots.insert(ctx, exports));
    let module_obj = qjs::JS_NewObject(ctx);
    let exports_dup = qjs::JS_DupValue(ctx, exports);
    qjs::JS_SetPropertyStr(ctx, module_obj, c"exports".as_ptr(), exports_dup);
    let module_root = with_bridge_roots(|ctx, roots| roots.insert(ctx, module_obj));

    let mut out_exports = exports_root;
    let status = raster_v8_run_module_init(
        context_state,
        module,
        exports_root,
        module_root,
        &mut out_exports,
    );
    if status != 0 {
        return Err(format!("V8 module init failed with status {}", status));
    }

    with_bridge_roots(|ctx, roots| {
        roots.drop_root(ctx, module_root);
        if out_exports != exports_root {
            roots.drop_root(ctx, out_exports);
        }
    });
    Ok(())
}
