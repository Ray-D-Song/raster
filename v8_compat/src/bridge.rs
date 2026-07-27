use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::mem;
use std::os::raw::{c_char, c_int, c_void};
use std::panic::{catch_unwind, AssertUnwindSafe, UnwindSafe};
use std::ptr;
use std::sync::OnceLock;

use rquickjs::qjs::{self, JSContext, JSValue};

use crate::abi::{NAPI_API_VERSION, NODE_MODULE_VERSION};
use crate::js_ops::new_v8_object;
use crate::module_loader::{ensure_context_for_ctx, ensure_isolate_for_runtime};
use crate::root::RootTable;

#[repr(C)]
#[derive(PartialEq)]
pub enum RasterV8Status {
    Ok = 0,
    Exception = 1,
    Unsupported = 2,
    WrongThread = 3,
    Error = 4,
}

#[repr(C)]
pub struct RasterV8ContextState {
    _private: [u8; 0],
}

#[repr(C)]
pub struct RasterV8IsolateState {
    _private: [u8; 0],
}

type RootDupFn = unsafe extern "C" fn(u64, *mut u64) -> RasterV8Status;
type RootDropFn = unsafe extern "C" fn(u64) -> RasterV8Status;
type RootFromJsFn =
    unsafe extern "C" fn(*mut RasterV8ContextState, u64, *mut u64) -> RasterV8Status;
type RootToJsFn =
    unsafe extern "C" fn(*mut RasterV8ContextState, u64, *mut u64) -> RasterV8Status;
type ThrowTypeErrorFn = unsafe extern "C" fn(*mut RasterV8ContextState, *const c_char) -> RasterV8Status;
type FatalFn = unsafe extern "C" fn(*const c_char, *const c_char);
type StringNewUtf8Fn =
    unsafe extern "C" fn(*mut RasterV8ContextState, *const c_char, i32, *mut u64) -> RasterV8Status;
type ObjectNewFn = unsafe extern "C" fn(*mut RasterV8ContextState, *mut u64) -> RasterV8Status;
type ObjectSetFn = unsafe extern "C" fn(
    *mut RasterV8ContextState,
    u64,
    u64,
    u64,
) -> RasterV8Status;
type FunctionTemplateNewFn = unsafe extern "C" fn(
    *mut RasterV8ContextState,
    u32,
    *mut std::ffi::c_void,
    u64,
    *mut u32,
) -> RasterV8Status;
type FunctionTemplateGetFunctionFn =
    unsafe extern "C" fn(*mut RasterV8ContextState, u32, *mut u64) -> RasterV8Status;
type DispatchFunctionFn = unsafe extern "C" fn(
    *mut RasterV8ContextState,
    u32,
    u64,
    u64,
    *const u64,
    i32,
    *mut u64,
) -> RasterV8Status;
type RunModuleInitFn = unsafe extern "C" fn(
    *mut RasterV8ContextState,
    unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, *mut std::ffi::c_void),
    u64,
    u64,
    *mut u64,
) -> RasterV8Status;
type ObjectGetFn = unsafe extern "C" fn(*mut RasterV8ContextState, u64, u64, *mut u64) -> RasterV8Status;
type ObjectGetIndexFn =
    unsafe extern "C" fn(*mut RasterV8ContextState, u64, u32, *mut u64) -> RasterV8Status;
type ObjectSetIndexFn =
    unsafe extern "C" fn(*mut RasterV8ContextState, u64, u32, u64) -> RasterV8Status;
type ObjectDefineOwnPropertyFn = unsafe extern "C" fn(
    *mut RasterV8ContextState,
    u64,
    u64,
    u64,
    i32,
    *mut bool,
) -> RasterV8Status;
type ObjectHasOwnPropertyFn =
    unsafe extern "C" fn(*mut RasterV8ContextState, u64, u64, *mut bool) -> RasterV8Status;
type ObjectGetPrototypeFn =
    unsafe extern "C" fn(*mut RasterV8ContextState, u64, *mut u64) -> RasterV8Status;
type ArrayNewFn = unsafe extern "C" fn(*mut RasterV8ContextState, i32, *mut u64) -> RasterV8Status;
type NumberNewFn = unsafe extern "C" fn(*mut RasterV8ContextState, f64, *mut u64) -> RasterV8Status;
type BigIntNewFn = unsafe extern "C" fn(*mut RasterV8ContextState, i64, *mut u64) -> RasterV8Status;
type IntegerNewFn = unsafe extern "C" fn(*mut RasterV8ContextState, i32, *mut u64) -> RasterV8Status;
type StringNewLatin1Fn =
    unsafe extern "C" fn(*mut RasterV8ContextState, *const u8, i32, *mut u64) -> RasterV8Status;
type StringToUtf8Fn = unsafe extern "C" fn(
    *mut RasterV8ContextState,
    u64,
    *mut *mut c_char,
    *mut usize,
) -> RasterV8Status;
type StringFreeUtf8Fn = unsafe extern "C" fn(*mut RasterV8ContextState, *mut c_char) -> RasterV8Status;
type FunctionCallFn = unsafe extern "C" fn(
    *mut RasterV8ContextState,
    u64,
    u64,
    i32,
    *const u64,
    *mut u64,
) -> RasterV8Status;
type ThrowValueFn = unsafe extern "C" fn(*mut RasterV8ContextState, u64) -> RasterV8Status;
type NewExceptionFn =
    unsafe extern "C" fn(*mut RasterV8ContextState, u64, i32, *mut u64) -> RasterV8Status;
type ExternalNewFn = unsafe extern "C" fn(*mut RasterV8ContextState, *mut std::ffi::c_void, *mut u64)
    -> RasterV8Status;
type InternalFieldSetFn =
    unsafe extern "C" fn(*mut RasterV8ContextState, u64, i32, *mut std::ffi::c_void) -> RasterV8Status;
type InternalFieldGetFn = unsafe extern "C" fn(
    *mut RasterV8ContextState,
    u64,
    i32,
    *mut *mut std::ffi::c_void,
) -> RasterV8Status;
type SymbolIteratorFn = unsafe extern "C" fn(*mut RasterV8ContextState, *mut u64) -> RasterV8Status;
type GetCreationContextFn =
    unsafe extern "C" fn(*mut RasterV8ContextState, u64, *mut u64) -> RasterV8Status;
type RegisterWeakCallbackFn = unsafe extern "C" fn(
    *mut RasterV8ContextState,
    u64,
    *mut std::ffi::c_void,
    Option<unsafe extern "C" fn(*const std::ffi::c_void, i32)>,
) -> RasterV8Status;
type GetContextRootFn = unsafe extern "C" fn(*mut RasterV8ContextState, *mut u64) -> RasterV8Status;

#[repr(C)]
pub struct RasterV8BridgeV1 {
    pub version: u32,
    pub node_module_version: u32,
    pub root_dup: Option<RootDupFn>,
    pub root_drop: Option<RootDropFn>,
    pub root_from_js: Option<RootFromJsFn>,
    pub root_to_js: Option<RootToJsFn>,
    pub throw_type_error: Option<ThrowTypeErrorFn>,
    pub fatal: Option<FatalFn>,
    pub string_new_utf8: Option<StringNewUtf8Fn>,
    pub object_new: Option<ObjectNewFn>,
    pub object_set: Option<ObjectSetFn>,
    pub function_template_new: Option<FunctionTemplateNewFn>,
    pub function_template_get_function: Option<FunctionTemplateGetFunctionFn>,
    pub dispatch_function: Option<DispatchFunctionFn>,
    pub run_module_init: Option<RunModuleInitFn>,
    pub object_get: Option<ObjectGetFn>,
    pub object_get_index: Option<ObjectGetIndexFn>,
    pub object_set_index: Option<ObjectSetIndexFn>,
    pub object_define_own_property: Option<ObjectDefineOwnPropertyFn>,
    pub object_has_own_property: Option<ObjectHasOwnPropertyFn>,
    pub object_get_prototype: Option<ObjectGetPrototypeFn>,
    pub array_new: Option<ArrayNewFn>,
    pub number_new: Option<NumberNewFn>,
    pub bigint_new: Option<BigIntNewFn>,
    pub integer_new: Option<IntegerNewFn>,
    pub string_new_latin1: Option<StringNewLatin1Fn>,
    pub string_to_utf8: Option<StringToUtf8Fn>,
    pub string_free_utf8: Option<StringFreeUtf8Fn>,
    pub function_call: Option<FunctionCallFn>,
    pub throw_value: Option<ThrowValueFn>,
    pub new_exception: Option<NewExceptionFn>,
    pub external_new: Option<ExternalNewFn>,
    pub internal_field_set: Option<InternalFieldSetFn>,
    pub internal_field_get: Option<InternalFieldGetFn>,
    pub symbol_iterator: Option<SymbolIteratorFn>,
    pub get_creation_context: Option<GetCreationContextFn>,
    pub register_weak_callback: Option<RegisterWeakCallbackFn>,
    pub get_context_root: Option<GetContextRootFn>,
}

struct SendPtr<T>(*mut T);
unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}

pub struct BridgeState {
    pub roots: RootTable,
    ctx: SendPtr<JSContext>,
}

impl BridgeState {
    pub(crate) fn ctx_ptr(&self) -> *mut JSContext {
        self.ctx.0
    }
}

thread_local! {
    static BRIDGE_STATE: RefCell<Option<BridgeState>> = const { RefCell::new(None) };
}

pub(crate) fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut BridgeState) -> R,
{
    BRIDGE_STATE.with(|cell| {
        let mut guard = cell.borrow_mut();
        let state = guard.as_mut().expect("v8 bridge not initialized");
        f(state)
    })
}

pub(crate) fn with_state_ref<F, R>(f: F) -> R
where
    F: FnOnce(&BridgeState) -> R,
{
    BRIDGE_STATE.with(|cell| {
        let guard = cell.borrow();
        let state = guard.as_ref().expect("v8 bridge not initialized");
        f(state)
    })
}

fn catch_panic<F>(f: F) -> RasterV8Status
where
    F: FnOnce() -> RasterV8Status + UnwindSafe,
{
    match catch_unwind(f) {
        Ok(status) => status,
        Err(_) => RasterV8Status::Error,
    }
}

pub(crate) fn resolve_constructor_root(
    state: &BridgeState,
    func_root: u64,
) -> Option<(u64, rquickjs::qjs::JSValue)> {
    let ctx = state.ctx_ptr();
    if let Some(func) = state.roots.get(func_root) {
        if unsafe { rquickjs::qjs::JS_IsConstructor(ctx, func) } {
            return Some((func_root, func));
        }
    }
    let function_id = ROOT_FUNCTION_IDS.with(|handles| handles.borrow().get(&func_root).copied());
    if let Some(function_id) = function_id {
        let cached_root = FUNCTION_ROOTS.with(|handles| handles.borrow().get(&function_id).copied());
        if let Some(root) = cached_root {
            if let Some(func) = state.roots.get(root) {
                if unsafe { rquickjs::qjs::JS_IsConstructor(ctx, func) } {
                    return Some((root, func));
                }
            }
        }
    }
    None
}

pub(crate) fn function_root_for_id(function_id: u32) -> Option<u64> {
    FUNCTION_ROOTS.with(|handles| handles.borrow().get(&function_id).copied())
}

pub(crate) fn function_id_for_root(root: u64) -> Option<u32> {
    ROOT_FUNCTION_IDS.with(|handles| handles.borrow().get(&root).copied())
}

#[no_mangle]
pub unsafe extern "C" fn raster_v8_function_id_for_root(
    _ctx: *mut RasterV8ContextState,
    root_id: u64,
    out_function_id: *mut u32,
) -> RasterV8Status {
    if out_function_id.is_null() {
        return RasterV8Status::Error;
    }
    if let Some(function_id) = function_id_for_root(root_id) {
        *out_function_id = function_id;
        RasterV8Status::Ok
    } else {
        RasterV8Status::Error
    }
}

#[no_mangle]
pub unsafe extern "C" fn raster_v8_function_root_for_id(
    _ctx: *mut RasterV8ContextState,
    function_id: u32,
    out_root_id: *mut u64,
) -> RasterV8Status {
    if out_root_id.is_null() {
        return RasterV8Status::Error;
    }
    if let Some(root) = function_root_for_id(function_id) {
        *out_root_id = root;
        RasterV8Status::Ok
    } else {
        RasterV8Status::Error
    }
}

unsafe extern "C" fn root_dup(id: u64, out: *mut u64) -> RasterV8Status {
    catch_panic(AssertUnwindSafe(|| {
        if out.is_null() {
            return RasterV8Status::Error;
        }
        with_state(|state| {
            match state.roots.dup(state.ctx_ptr(), id) {
                Some(new_id) => {
                    ROOT_FUNCTION_IDS.with(|handles| {
                        let mut map = handles.borrow_mut();
                        if let Some(function_id) = map.remove(&id) {
                            map.insert(new_id, function_id);
                            FUNCTION_ROOTS.with(|fr| {
                                fr.borrow_mut().insert(function_id, new_id);
                            });
                        }
                    });
                    *out = new_id;
                    RasterV8Status::Ok
                }
                None => RasterV8Status::Error,
            }
        })
    }))
}

unsafe extern "C" fn root_drop(id: u64) -> RasterV8Status {
    catch_panic(AssertUnwindSafe(|| {
        with_state(|state| {
            state.roots.drop_root(state.ctx_ptr(), id);
            RasterV8Status::Ok
        })
    }))
}

unsafe extern "C" fn string_new_utf8(
    _ctx: *mut RasterV8ContextState,
    data: *const c_char,
    length: i32,
    out: *mut u64,
) -> RasterV8Status {
    catch_panic(AssertUnwindSafe(|| {
        if data.is_null() || out.is_null() {
            return RasterV8Status::Error;
        }
        with_state(|state| {
            let bytes = if length < 0 {
                std::ffi::CStr::from_ptr(data).to_bytes()
            } else {
                std::slice::from_raw_parts(data as *const u8, length as usize)
            };
            let js = qjs::JS_NewStringLen(
                state.ctx_ptr(),
                bytes.as_ptr() as *const c_char,
                bytes.len() as u64,
            );
            *out = state.roots.insert(state.ctx_ptr(), js);
            RasterV8Status::Ok
        })
    }))
}

unsafe extern "C" fn object_new(_ctx: *mut RasterV8ContextState, out: *mut u64) -> RasterV8Status {
    catch_panic(AssertUnwindSafe(|| {
        if out.is_null() {
            return RasterV8Status::Error;
        }
        with_state(|state| {
            let obj = crate::js_ops::new_v8_object(state.ctx_ptr());
            *out = state.roots.insert(state.ctx_ptr(), obj);
            RasterV8Status::Ok
        })
    }))
}

unsafe extern "C" fn object_set(
    _ctx: *mut RasterV8ContextState,
    object_root: u64,
    key_root: u64,
    value_root: u64,
) -> RasterV8Status {
    catch_panic(AssertUnwindSafe(|| {
        with_state(|state| {
            let Some(obj) = state.roots.get(object_root) else {
                return RasterV8Status::Error;
            };
            let Some(key) = state.roots.get(key_root) else {
                return RasterV8Status::Error;
            };
            let Some(val) = state.roots.get(value_root) else {
                return RasterV8Status::Error;
            };
            let atom = qjs::JS_ValueToAtom(state.ctx_ptr(), key);
            if atom == 0 {
                return RasterV8Status::Error;
            }
            let rc = qjs::JS_SetProperty(state.ctx_ptr(), obj, atom, qjs::JS_DupValue(state.ctx_ptr(), val));
            qjs::JS_FreeAtom(state.ctx_ptr(), atom);
            if rc < 0 {
                return RasterV8Status::Exception;
            }
            RasterV8Status::Ok
        })
    }))
}

unsafe extern "C" fn throw_type_error(
    _ctx: *mut RasterV8ContextState,
    message: *const c_char,
) -> RasterV8Status {
    catch_panic(AssertUnwindSafe(|| {
        with_state(|state| {
            let msg = if message.is_null() {
                "V8 ABI error"
            } else {
                std::ffi::CStr::from_ptr(message).to_str().unwrap_or("V8 ABI error")
            };
            let err = qjs::JS_NewError(state.ctx_ptr());
            qjs::JS_SetPropertyStr(
                state.ctx_ptr(),
                err,
                c"message".as_ptr(),
                qjs::JS_NewStringLen(state.ctx_ptr(), msg.as_ptr() as *const c_char, msg.len() as u64),
            );
            qjs::JS_Throw(state.ctx_ptr(), err);
            RasterV8Status::Exception
        })
    }))
}

unsafe extern "C" fn fatal(api: *const c_char, message: *const c_char) {
    let api = if api.is_null() {
        "v8"
    } else {
        std::ffi::CStr::from_ptr(api).to_str().unwrap_or("v8")
    };
    let message = if message.is_null() {
        "fatal ABI violation"
    } else {
        std::ffi::CStr::from_ptr(message).to_str().unwrap_or("fatal ABI violation")
    };
    panic!("V8 ABI fatal in {api}: {message}");
}

unsafe extern "C" fn unsupported_fn() -> RasterV8Status {
    RasterV8Status::Unsupported
}

unsafe extern "C" fn unsupported_template_new(
    _ctx: *mut RasterV8ContextState,
    _template_id: u32,
    _callback: *mut std::ffi::c_void,
    _data_root: u64,
    _out: *mut u32,
) -> RasterV8Status {
    RasterV8Status::Unsupported
}

thread_local! {
    static FUNCTION_ROOTS: RefCell<HashMap<u32, u64>> = RefCell::new(HashMap::new());
    static ROOT_FUNCTION_IDS: RefCell<HashMap<u64, u32>> = RefCell::new(HashMap::new());
}

unsafe fn make_v8_constructor_from_id(
    ctx: *mut JSContext,
    state: &BridgeState,
    function_id: u32,
) -> (JSValue, u64) {
    let func = qjs::JS_NewCFunction2(
        ctx,
        mem::transmute::<qjs::JSCFunctionMagic, qjs::JSCFunction>(Some(v8_js_cfunc_magic)),
        c"v8".as_ptr(),
        0,
        qjs::JSCFunctionEnum_JS_CFUNC_constructor_or_func_magic,
        function_id as i32,
    );
    qjs::JS_SetConstructorBit(ctx, func, true);
    let root = state.roots.insert(ctx, func);
    FUNCTION_ROOTS.with(|handles| handles.borrow_mut().insert(function_id, root));
    ROOT_FUNCTION_IDS.with(|handles| handles.borrow_mut().insert(root, function_id));
    (func, root)
}

unsafe extern "C" fn v8_js_method_magic(
    ctx: *mut JSContext,
    this_val: JSValue,
    argc: c_int,
    argv: *mut JSValue,
    magic: c_int,
) -> JSValue {
    v8_js_trampoline_inner(ctx, this_val, argc, argv, magic, false)
}

unsafe fn make_v8_method_from_id(
    ctx: *mut JSContext,
    state: &BridgeState,
    function_id: u32,
) -> (JSValue, u64) {
    let func = qjs::JS_NewCFunction2(
        ctx,
        mem::transmute::<qjs::JSCFunctionMagic, qjs::JSCFunction>(Some(v8_js_method_magic)),
        c"v8".as_ptr(),
        0,
        qjs::JSCFunctionEnum_JS_CFUNC_generic_magic,
        function_id as i32,
    );
    let root = state.roots.insert(ctx, func);
    FUNCTION_ROOTS.with(|handles| handles.borrow_mut().insert(function_id, root));
    ROOT_FUNCTION_IDS.with(|handles| handles.borrow_mut().insert(root, function_id));
    (func, root)
}

unsafe fn make_accessor_getter(ctx: *mut JSContext, accessor_id: u32) -> JSValue {
    qjs::JS_NewCFunction2(
        ctx,
        mem::transmute::<qjs::JSCFunctionMagic, qjs::JSCFunction>(Some(v8_accessor_getter)),
        c"get".as_ptr(),
        0,
        qjs::JSCFunctionEnum_JS_CFUNC_generic_magic,
        accessor_id as i32,
    )
}

unsafe extern "C" fn v8_accessor_getter(
    ctx: *mut JSContext,
    this_val: JSValue,
    _argc: c_int,
    _argv: *mut JSValue,
    magic: c_int,
) -> JSValue {
    let rt = unsafe { qjs::JS_GetRuntime(ctx) };
    let isolate = ensure_isolate_for_runtime(rt);
    let context_state = ensure_context_for_ctx(ctx);
    unsafe {
        raster_v8_set_current_context(context_state as *mut RasterV8ContextState);
        raster_v8_set_current_isolate(isolate as *mut RasterV8IsolateState);
    }
    let accessor_id = magic as u32;
    let receiver_root = BRIDGE_STATE.with(|cell| {
        let guard = cell.borrow();
        let state = guard.as_ref().expect("v8 bridge not initialized");
        state.roots.insert(ctx, qjs::JS_DupValue(ctx, this_val))
    });
    let embedder = crate::js_ops::embedder_ptr_for_object(ctx, this_val, receiver_root, 0);
    let _embedder_scope = crate::js_ops::EmbedderScopeGuard::enter();
    if embedder != 0 {
        crate::js_ops::set_embedder_field0_in_frame(embedder);
    }
    let mut result_root = 0u64;
    let status = unsafe {
        raster_v8_dispatch_accessor(
            accessor_id,
            receiver_root,
            embedder as *mut c_void,
            &mut result_root,
        )
    };
    BRIDGE_STATE.with(|cell| {
        let guard = cell.borrow();
        let state = guard.as_ref().expect("v8 bridge not initialized");
        state.roots.drop_root(ctx, receiver_root);
    });
    if status != RasterV8Status::Ok || result_root == 0 {
        return qjs::JS_UNDEFINED;
    }
    let result = BRIDGE_STATE.with(|cell| {
        let guard = cell.borrow();
        let state = guard.as_ref().expect("v8 bridge not initialized");
        state
            .roots
            .get(result_root)
            .map(|v| unsafe { qjs::JS_DupValue(ctx, v) })
            .unwrap_or(qjs::JS_UNDEFINED)
    });
    BRIDGE_STATE.with(|cell| {
        let guard = cell.borrow();
        let state = guard.as_ref().expect("v8 bridge not initialized");
        state.roots.drop_root(ctx, result_root);
    });
    result
}

unsafe fn install_function_prototype(
    ctx: *mut JSContext,
    state: &BridgeState,
    template_id: u32,
    func: JSValue,
) {
    let proto_template_id = raster_v8_function_prototype_template_id(template_id);
    if proto_template_id == 0 {
        let proto = qjs::JS_NewObject(ctx);
        qjs::JS_SetConstructor(ctx, func, proto);
        return;
    }
    let proto = qjs::JS_NewObject(ctx);
    let count = raster_v8_object_template_property_count(proto_template_id);
    for i in 0..count {
        let mut key_root = 0u64;
        let mut child_template_id = 0u32;
        if raster_v8_object_template_property_at(
            proto_template_id,
            i,
            &mut key_root,
            &mut child_template_id,
        ) == 0
        {
            continue;
        }
        let child_function_id = raster_v8_register_function_for_template(child_template_id);
        let (child_fn, _) = make_v8_method_from_id(ctx, state, child_function_id);
        if let Some(key) = state.roots.get(key_root) {
            let atom = qjs::JS_ValueToAtom(ctx, key);
            if atom != 0 {
                qjs::JS_SetProperty(ctx, proto, atom, child_fn);
                qjs::JS_FreeAtom(ctx, atom);
            }
        }
    }
    let native_count = raster_v8_object_template_native_property_count(proto_template_id);
    for i in 0..native_count {
        let mut name_root = 0u64;
        let mut accessor_id = 0u32;
        if raster_v8_object_template_native_property_at(
            proto_template_id,
            i,
            &mut name_root,
            &mut accessor_id,
        ) == 0
        {
            continue;
        }
        let getter = make_accessor_getter(ctx, accessor_id);
        if let Some(key) = state.roots.get(name_root) {
            let atom = qjs::JS_ValueToAtom(ctx, key);
            if atom != 0 {
                qjs::JS_DefinePropertyGetSet(
                    ctx,
                    proto,
                    atom,
                    getter,
                    qjs::JS_UNDEFINED,
                    (qjs::JS_PROP_ENUMERABLE | qjs::JS_PROP_CONFIGURABLE) as i32,
                );
                qjs::JS_FreeAtom(ctx, atom);
            }
        }
    }
    let proto_root = state.roots.insert(ctx, unsafe { qjs::JS_DupValue(ctx, proto) });
    unsafe {
        raster_v8_set_function_template_prototype_root(template_id, proto_root);
    }
    qjs::JS_SetConstructor(ctx, func, proto);
}

unsafe extern "C" fn function_template_get_function(
    _ctx: *mut RasterV8ContextState,
    function_id: u32,
    out: *mut u64,
) -> RasterV8Status {
    catch_panic(AssertUnwindSafe(|| {
        if out.is_null() {
            return RasterV8Status::Error;
        }
        with_state(|state| {
            let ctx = state.ctx_ptr();
            let cached_root = FUNCTION_ROOTS.with(|handles| {
                handles
                    .borrow()
                    .get(&function_id)
                    .copied()
                    .filter(|&root| state.roots.get(root).is_some())
            });
            let root = if let Some(root) = cached_root {
                ROOT_FUNCTION_IDS.with(|handles| {
                    handles.borrow_mut().insert(root, function_id);
                });
                root
            } else {
                let (func, root) = make_v8_constructor_from_id(ctx, state, function_id);
                let template_id = unsafe { raster_v8_function_template_id(function_id) };
                install_function_prototype(ctx, state, template_id, func);
                root
            };
            *out = root;
            RasterV8Status::Ok
        })
    }))
}

unsafe extern "C" fn dispatch_function(
    _ctx: *mut RasterV8ContextState,
    function_id: u32,
    receiver_root: u64,
    _new_target_root: u64,
    arg_root_ids: *const u64,
    argc: i32,
    out_result_root: *mut u64,
) -> RasterV8Status {
    catch_panic(AssertUnwindSafe(|| {
        if out_result_root.is_null() {
            return RasterV8Status::Error;
        }
        let args = if arg_root_ids.is_null() || argc <= 0 {
            &[]
        } else {
            std::slice::from_raw_parts(arg_root_ids, argc as usize)
        };
        let mut result_root = 0u64;
        let status = raster_v8_dispatch_callback(
            function_id,
            receiver_root,
            _new_target_root,
            args.as_ptr(),
            argc,
            &mut result_root,
        );
        if status == RasterV8Status::Ok {
            *out_result_root = result_root;
        }
        status
    }))
}

unsafe extern "C" fn v8_js_cfunc_magic(
    ctx: *mut JSContext,
    this_val: JSValue,
    argc: c_int,
    argv: *mut JSValue,
    magic: c_int,
) -> JSValue {
    v8_js_trampoline_inner(ctx, this_val, argc, argv, magic, true)
}

unsafe extern "C" fn v8_js_trampoline(
    ctx: *mut JSContext,
    this_val: JSValue,
    argc: c_int,
    argv: *mut JSValue,
    magic: c_int,
    _func_data: *mut JSValue,
) -> JSValue {
    v8_js_trampoline_inner(ctx, this_val, argc, argv, magic, false)
}

unsafe fn v8_js_trampoline_inner(
    ctx: *mut JSContext,
    this_val: JSValue,
    argc: c_int,
    argv: *mut JSValue,
    magic: c_int,
    use_constructor_semantics: bool,
) -> JSValue {
    let rt = unsafe { qjs::JS_GetRuntime(ctx) };
    let isolate = ensure_isolate_for_runtime(rt);
    let context_state = ensure_context_for_ctx(ctx);
    unsafe {
        raster_v8_set_current_context(context_state as *mut RasterV8ContextState);
        raster_v8_set_current_isolate(isolate as *mut RasterV8IsolateState);
    }

    let function_id = magic as u32;
    // QuickJS passes `undefined` for ordinary calls on JS_CFUNC_constructor_or_func_magic.
    // For C function constructors it passes new_target (= the ctor function) as `this`.
    let is_construct = use_constructor_semantics
        && unsafe { !qjs::JS_IsUndefined(this_val) };

    let (receiver_root, new_target_root, instance_val) = if is_construct {
        let template_id = unsafe { raster_v8_function_template_id(function_id) };
        let field_count = unsafe { raster_v8_instance_internal_field_count(template_id) };
        let ctor_is_new_target =
            unsafe { qjs::JS_IsFunction(ctx, this_val) };
        let proto = if ctor_is_new_target {
            let installed = BRIDGE_STATE.with(|cell| {
                let guard = cell.borrow();
                let state = guard.as_ref().expect("v8 bridge not initialized");
                let proto_root = unsafe { raster_v8_function_template_prototype_root(template_id) };
                proto_root
                    .ne(&0)
                    .then(|| proto_root)
                    .and_then(|proto_root| state.roots.get(proto_root))
                    .map(|v| unsafe { qjs::JS_DupValue(ctx, v) })
            });
            installed.unwrap_or_else(|| unsafe {
                qjs::JS_GetPropertyStr(ctx, this_val, c"prototype".as_ptr())
            })
        } else {
            BRIDGE_STATE.with(|cell| {
                let guard = cell.borrow();
                let state = guard.as_ref().expect("v8 bridge not initialized");
                let proto_root = unsafe { raster_v8_function_template_prototype_root(template_id) };
                if proto_root != 0 {
                    state.roots.get(proto_root).map(|v| unsafe { qjs::JS_DupValue(ctx, v) })
                } else {
                    None
                }
            })
            .unwrap_or(qjs::JS_UNDEFINED)
        };
        let instance = unsafe {
            if ctor_is_new_target {
                let obj = if qjs::JS_IsUndefined(proto) {
                    new_v8_object(ctx)
                } else {
                    let obj = qjs::JS_NewObject(ctx);
                    qjs::JS_SetPrototype(ctx, obj, proto);
                    qjs::JS_FreeValue(ctx, proto);
                    obj
                };
                obj
            } else {
                qjs::JS_DupValue(ctx, this_val)
            }
        };
        let receiver_root = BRIDGE_STATE.with(|cell| {
            let guard = cell.borrow();
            let state = guard.as_ref().expect("v8 bridge not initialized");
            state.roots.insert(ctx, qjs::JS_DupValue(ctx, instance))
        });
        if field_count > 0 {
            unsafe {
                raster_v8_object_reserve_internal_fields(
                    context_state as *mut RasterV8ContextState,
                    receiver_root,
                    field_count,
                );
            }
        }
        let new_target_root = BRIDGE_STATE.with(|cell| {
            let guard = cell.borrow();
            let state = guard.as_ref().expect("v8 bridge not initialized");
            if ctor_is_new_target {
                state.roots.insert(ctx, qjs::JS_DupValue(ctx, this_val))
            } else {
                function_root_for_id(function_id)
                    .and_then(|root| state.roots.get(root))
                    .map(|func| state.roots.insert(ctx, qjs::JS_DupValue(ctx, func)))
                    .unwrap_or(0)
            }
        });
        (receiver_root, new_target_root, Some(instance))
    } else {
        let receiver_root = BRIDGE_STATE.with(|cell| {
            let guard = cell.borrow();
            let state = guard.as_ref().expect("v8 bridge not initialized");
            state.roots.insert(ctx, qjs::JS_DupValue(ctx, this_val))
        });
        (receiver_root, 0, None)
    };

    let mut arg_roots = Vec::with_capacity(argc.max(0) as usize);
    if !argv.is_null() && argc > 0 {
        let slice = std::slice::from_raw_parts(argv, argc as usize);
        BRIDGE_STATE.with(|cell| {
            let guard = cell.borrow();
            let state = guard.as_ref().expect("v8 bridge not initialized");
            for &arg in slice {
                arg_roots.push(state.roots.insert(ctx, qjs::JS_DupValue(ctx, arg)));
            }
        });
    }
    let embedder_patch = None;
    let _embedder_scope = crate::js_ops::EmbedderScopeGuard::enter();
    let mut result_root = 0u64;
    let status = unsafe {
        raster_v8_dispatch_callback(
            function_id,
            receiver_root,
            new_target_root,
            arg_roots.as_ptr(),
            argc,
            &mut result_root,
        )
    };
    if let Some(saved) = embedder_patch {
        unsafe { crate::js_ops::restore_js_object_embedder_slot(this_val, saved) };
    }
    if status == RasterV8Status::Exception || unsafe { qjs::JS_HasException(ctx) } {
        BRIDGE_STATE.with(|cell| {
            let guard = cell.borrow();
            let state = guard.as_ref().expect("v8 bridge not initialized");
            state.roots.drop_root(ctx, receiver_root);
            if new_target_root != 0 {
                state.roots.drop_root(ctx, new_target_root);
            }
            for id in arg_roots {
                state.roots.drop_root(ctx, id);
            }
        });
        if let Some(instance) = instance_val {
            unsafe { qjs::JS_FreeValue(ctx, instance) };
        }
        return unsafe { qjs::JS_GetException(ctx) };
    }
    if status != RasterV8Status::Ok {
        return qjs::JS_UNDEFINED;
    }
    if let Some(instance) = instance_val {
        BRIDGE_STATE.with(|cell| {
            let guard = cell.borrow();
            let state = guard.as_ref().expect("v8 bridge not initialized");
            state.roots.drop_root(ctx, receiver_root);
            if new_target_root != 0 {
                state.roots.drop_root(ctx, new_target_root);
            }
            for id in arg_roots {
                state.roots.drop_root(ctx, id);
            }
        });
        if result_root != 0 && result_root != receiver_root {
            BRIDGE_STATE.with(|cell| {
                let guard = cell.borrow();
                let state = guard.as_ref().expect("v8 bridge not initialized");
                state.roots.drop_root(ctx, result_root);
            });
        }
        return instance;
    }
    if result_root != 0 && result_root != receiver_root {
        BRIDGE_STATE.with(|cell| {
            let guard = cell.borrow();
            let state = guard.as_ref().expect("v8 bridge not initialized");
            state.roots.drop_root(ctx, receiver_root);
            if new_target_root != 0 {
                state.roots.drop_root(ctx, new_target_root);
            }
            for id in arg_roots {
                state.roots.drop_root(ctx, id);
            }
        });
        let result = BRIDGE_STATE.with(|cell| {
            let guard = cell.borrow();
            let state = guard.as_ref().expect("v8 bridge not initialized");
            state
                .roots
                .get(result_root)
                .map(|v| unsafe { qjs::JS_DupValue(ctx, v) })
                .unwrap_or(qjs::JS_UNDEFINED)
        });
        BRIDGE_STATE.with(|cell| {
            let guard = cell.borrow();
            let state = guard.as_ref().expect("v8 bridge not initialized");
            state.roots.drop_root(ctx, result_root);
        });
        return result;
    }
    if result_root != 0 && result_root == receiver_root {
        let result = BRIDGE_STATE.with(|cell| {
            let guard = cell.borrow();
            let state = guard.as_ref().expect("v8 bridge not initialized");
            state
                .roots
                .get(receiver_root)
                .map(|v| unsafe { qjs::JS_DupValue(ctx, v) })
                .unwrap_or(qjs::JS_UNDEFINED)
        });
        BRIDGE_STATE.with(|cell| {
            let guard = cell.borrow();
            let state = guard.as_ref().expect("v8 bridge not initialized");
            state.roots.drop_root(ctx, receiver_root);
            if new_target_root != 0 {
                state.roots.drop_root(ctx, new_target_root);
            }
            for id in arg_roots {
                state.roots.drop_root(ctx, id);
            }
        });
        return result;
    }
    BRIDGE_STATE.with(|cell| {
        let guard = cell.borrow();
        let state = guard.as_ref().expect("v8 bridge not initialized");
        state.roots.drop_root(ctx, receiver_root);
        if new_target_root != 0 {
            state.roots.drop_root(ctx, new_target_root);
        }
        for id in arg_roots {
            state.roots.drop_root(ctx, id);
        }
    });
    if result_root != 0 {
        let result = BRIDGE_STATE.with(|cell| {
            let guard = cell.borrow();
            let state = guard.as_ref().expect("v8 bridge not initialized");
            state
                .roots
                .get(result_root)
                .map(|v| unsafe { qjs::JS_DupValue(ctx, v) })
                .unwrap_or(qjs::JS_UNDEFINED)
        });
        BRIDGE_STATE.with(|cell| {
            let guard = cell.borrow();
            let state = guard.as_ref().expect("v8 bridge not initialized");
            state.roots.drop_root(ctx, result_root);
        });
        return result;
    }
    qjs::JS_UNDEFINED
}

extern "C" {
    fn raster_v8_bind_bridge(bridge: *const RasterV8BridgeV1);
    fn raster_v8_set_oddball_root_fn(
        callback: unsafe extern "C" fn(
            *mut RasterV8ContextState,
            c_int,
            *mut u64,
        ) -> RasterV8Status,
    );
    fn raster_v8_force_link();
    fn raster_v8_set_current_context(ctx: *mut RasterV8ContextState);
    fn raster_v8_set_current_isolate(isolate: *mut RasterV8IsolateState);
    fn raster_v8_dispatch_callback(
        function_id: u32,
        receiver_root: u64,
        new_target_root: u64,
        arg_roots: *const u64,
        argc: i32,
        out_result_root: *mut u64,
    ) -> RasterV8Status;
    fn raster_v8_function_template_id(function_id: u32) -> u32;
    fn raster_v8_instance_internal_field_count(template_id: u32) -> i32;
    fn raster_v8_function_prototype_template_id(template_id: u32) -> u32;
    fn raster_v8_set_function_template_prototype_root(template_id: u32, root_id: u64);
    fn raster_v8_function_template_prototype_root(template_id: u32) -> u64;
    fn raster_v8_object_template_property_count(object_template_id: u32) -> usize;
    fn raster_v8_object_template_property_at(
        object_template_id: u32,
        index: usize,
        key_root: *mut u64,
        value_template_id: *mut u32,
    ) -> i32;
    fn raster_v8_register_function_for_template(template_id: u32) -> u32;
    fn raster_v8_object_template_native_property_count(object_template_id: u32) -> usize;
    fn raster_v8_object_template_native_property_at(
        object_template_id: u32,
        index: usize,
        name_root: *mut u64,
        accessor_id: *mut u32,
    ) -> i32;
    fn raster_v8_dispatch_accessor(
        accessor_id: u32,
        receiver_root: u64,
        embedder_override: *mut c_void,
        out_result_root: *mut u64,
    ) -> RasterV8Status;
    fn raster_v8_object_reserve_internal_fields(
        ctx: *mut RasterV8ContextState,
        object_root: u64,
        count: i32,
    ) -> RasterV8Status;
}

static BRIDGE_VTABLE: OnceLock<RasterV8BridgeV1> = OnceLock::new();

#[used]
static FORCE_V8_SHIM_LINK: unsafe extern "C" fn() = raster_v8_force_link;

/// Prevent the linker from dead-stripping the C++ V8 shim.
pub fn ensure_shim_linked() {
    unsafe {
        raster_v8_force_link();
    }
}

pub fn bind_bridge(ctx: *mut JSContext) {
    ensure_shim_linked();
    BRIDGE_STATE.with(|cell| {
        *cell.borrow_mut() = Some(BridgeState {
            roots: RootTable::new(),
            ctx: SendPtr(ctx),
        });
    });

    let bridge = BRIDGE_VTABLE.get_or_init(|| RasterV8BridgeV1 {
        version: 1,
        node_module_version: NODE_MODULE_VERSION as u32,
        root_dup: Some(root_dup),
        root_drop: Some(root_drop),
        root_from_js: None,
        root_to_js: None,
        throw_type_error: Some(throw_type_error),
        fatal: Some(fatal),
        string_new_utf8: Some(string_new_utf8),
        object_new: Some(object_new),
        object_set: Some(object_set),
        function_template_new: Some(unsupported_template_new),
        function_template_get_function: Some(function_template_get_function),
        dispatch_function: Some(dispatch_function),
        run_module_init: None,
        object_get: Some(crate::js_ops::object_get),
        object_get_index: Some(crate::js_ops::object_get_index),
        object_set_index: Some(crate::js_ops::object_set_index),
        object_define_own_property: Some(crate::js_ops::object_define_own_property),
        object_has_own_property: Some(crate::js_ops::object_has_own_property),
        object_get_prototype: Some(crate::js_ops::object_get_prototype),
        array_new: Some(crate::js_ops::array_new),
        number_new: Some(crate::js_ops::number_new),
        bigint_new: Some(crate::js_ops::bigint_new),
        integer_new: Some(crate::js_ops::integer_new),
        string_new_latin1: Some(crate::js_ops::string_new_latin1),
        string_to_utf8: Some(crate::js_ops::string_to_utf8),
        string_free_utf8: Some(crate::js_ops::string_free_utf8),
        function_call: Some(crate::js_ops::function_call),
        throw_value: Some(crate::js_ops::throw_value),
        new_exception: Some(crate::js_ops::new_exception),
        external_new: Some(crate::js_ops::external_new),
        internal_field_set: Some(crate::js_ops::internal_field_set),
        internal_field_get: Some(crate::js_ops::internal_field_get),
        symbol_iterator: Some(crate::js_ops::symbol_iterator),
        get_creation_context: Some(crate::js_ops::get_creation_context),
        register_weak_callback: Some(crate::js_ops::register_weak_callback),
        get_context_root: Some(crate::js_ops::get_context_root),
    });

    unsafe {
        raster_v8_bind_bridge(bridge);
        raster_v8_set_oddball_root_fn(crate::js_ops::oddball_root);
    }
}

/// Release rooted JS values before the QuickJS runtime is torn down.
pub fn prepare_shutdown(ctx: *mut JSContext) {
    FUNCTION_ROOTS.with(|handles| {
        let ids: Vec<u64> = handles.borrow().values().copied().collect();
        BRIDGE_STATE.with(|cell| {
            if let Some(state) = cell.borrow_mut().as_mut() {
                for id in ids {
                    state.roots.drop_root(state.ctx_ptr(), id);
                }
            }
        });
        handles.borrow_mut().clear();
    });
    crate::js_ops::prepare_shutdown(ctx);
    BRIDGE_STATE.with(|cell| {
        if let Some(mut state) = cell.borrow_mut().take() {
            state.roots.clear(ctx);
        }
    });
}

pub fn with_bridge_roots<F, R>(f: F) -> R
where
    F: FnOnce(*mut JSContext, &RootTable) -> R,
{
    BRIDGE_STATE.with(|cell| {
        let guard = cell.borrow();
        let state = guard.as_ref().expect("v8 bridge not initialized");
        f(state.ctx_ptr(), &state.roots)
    })
}

pub fn node_abi_version() -> u32 {
    NODE_MODULE_VERSION as u32
}

pub fn napi_node_version_u32() -> u32 {
    crate::abi::NAPI_NODE_VERSION_U32
}

pub fn napi_api_version() -> u32 {
    NAPI_API_VERSION
}
