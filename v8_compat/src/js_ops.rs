use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rquickjs::qjs::{self, JSContext, JSValue, JS_MKVAL, JS_TAG_INT};

use crate::bridge::{with_state, RasterV8Status};

fn new_int32(_ctx: *mut JSContext, val: i32) -> JSValue {
    unsafe { JS_MKVAL(JS_TAG_INT, val) }
}

thread_local! {
    static EMBEDDER_SCOPE_DEPTH: Cell<u32> = const { Cell::new(0) };
    static EMBEDDER_FIELD0_STACK: RefCell<Vec<usize>> = RefCell::new(Vec::new());
}

pub(crate) fn clear_embedder_field0_stack() {
    EMBEDDER_FIELD0_STACK.with(|stack| stack.borrow_mut().clear());
}

pub(crate) fn push_embedder_field0_frame() {
    EMBEDDER_FIELD0_STACK.with(|stack| stack.borrow_mut().push(0));
}

pub(crate) fn pop_embedder_field0_frame() {
    EMBEDDER_FIELD0_STACK.with(|stack| {
        stack.borrow_mut().pop();
    });
}

pub(crate) fn set_embedder_field0_in_frame(ptr: usize) {
    EMBEDDER_FIELD0_STACK.with(|stack| {
        if let Some(top) = stack.borrow_mut().last_mut() {
            *top = ptr;
        }
    });
}

pub(crate) fn current_embedder_field0_in_frame() -> usize {
    EMBEDDER_FIELD0_STACK.with(|stack| stack.borrow().last().copied().unwrap_or(0))
}

pub(crate) struct EmbedderScopeGuard;

impl EmbedderScopeGuard {
    pub fn enter() -> Self {
        EMBEDDER_SCOPE_DEPTH.with(|depth| {
            if depth.get() == 0 {
                clear_embedder_field0_stack();
            }
            depth.set(depth.get() + 1);
        });
        push_embedder_field0_frame();
        Self
    }
}

impl Drop for EmbedderScopeGuard {
    fn drop(&mut self) {
        pop_embedder_field0_frame();
        EMBEDDER_SCOPE_DEPTH.with(|depth| {
            let next = depth.get().saturating_sub(1);
            depth.set(next);
            if next == 0 {
                clear_embedder_field0_stack();
            }
        });
    }
}

static V8_OBJECT_CLASS: Lazy<Mutex<HashMap<usize, qjs::JSClassID>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static INTERNAL_FIELDS: Lazy<Mutex<HashMap<usize, Vec<usize>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static INTERNAL_FIELDS_BY_ROOT: Lazy<Mutex<HashMap<u64, Vec<usize>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static OBJECT_FIELD_COUNTS: Lazy<Mutex<HashMap<usize, usize>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static WEAK_CALLBACKS: Lazy<Mutex<HashMap<usize, WeakSlot>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[repr(C)]
pub struct WeakSlot {
    pub callback: Option<unsafe extern "C" fn(*const c_void, c_int)>,
    pub parameter: usize,
}

fn object_ptr_key(obj: JSValue) -> Option<usize> {
    if unsafe { !qjs::JS_IsObject(obj) } {
        return None;
    }
    Some(unsafe { qjs::JS_VALUE_GET_PTR(obj) as usize })
}

#[cfg(target_pointer_width = "64")]
const V8_EMBEDDER_SLOT_OFFSET: usize = 32;
#[cfg(not(target_pointer_width = "64"))]
const V8_EMBEDDER_SLOT_OFFSET: usize = 16;

fn internal_field_ptr_for_object(obj: JSValue, index: usize) -> Option<usize> {
    let key = object_ptr_key(obj)?;
    INTERNAL_FIELDS
        .lock()
        .get(&key)
        .and_then(|fields| fields.get(index).copied())
}

fn internal_field_ptr_for_root(root_id: u64, index: usize) -> Option<usize> {
    INTERNAL_FIELDS_BY_ROOT
        .lock()
        .get(&root_id)
        .and_then(|fields| fields.get(index).copied())
}

pub(crate) fn embedder_ptr_for_object(
    ctx: *mut JSContext,
    obj: JSValue,
    root_id: u64,
    index: usize,
) -> usize {
    if let Some(ptr) = internal_field_ptr_for_object(obj, index) {
        return ptr;
    }
    if let Some(ptr) = internal_field_ptr_for_root(root_id, index) {
        return ptr;
    }
    if index == 0 && unsafe { qjs::JS_IsObject(obj) } {
        let class_id = ensure_v8_object_class(ctx);
        let opaque = unsafe { qjs::JS_GetOpaque(obj, class_id) as usize };
        if opaque != 0 {
            return opaque;
        }
        let base = unsafe { qjs::JS_VALUE_GET_PTR(obj) as *mut u8 };
        let slot = unsafe { *(base.add(V8_EMBEDDER_SLOT_OFFSET) as *const usize) };
        if slot != 0 {
            return slot;
        }
    }
    0
}

fn object_is_raster_v8_class(ctx: *mut JSContext, obj: JSValue) -> bool {
    let class_id = ensure_v8_object_class(ctx);
    unsafe { qjs::JS_GetClassID(obj) == class_id }
}

pub(crate) unsafe fn patch_js_object_embedder_slot_for_root(
    ctx: *mut JSContext,
    obj: JSValue,
    root_id: u64,
    index: usize,
) -> Option<usize> {
    if unsafe { !qjs::JS_IsObject(obj) } || !object_is_raster_v8_class(ctx, obj) {
        return None;
    }
    let class_id = ensure_v8_object_class(ctx);
    let mut embedder = unsafe { qjs::JS_GetOpaque(obj, class_id) as usize };
    if embedder == 0 {
        embedder = internal_field_ptr_for_root(root_id, index)
            .or_else(|| internal_field_ptr_for_object(obj, index))
            .unwrap_or(0);
    }
    if embedder == 0 {
        return None;
    }
    let base = unsafe { qjs::JS_VALUE_GET_PTR(obj) as *mut u8 };
    let slot = unsafe {
        base.add(V8_EMBEDDER_SLOT_OFFSET) as *mut usize
    };
    let saved = unsafe { *slot };
    unsafe { *slot = embedder };
    Some(saved)
}

pub(crate) unsafe fn patch_js_object_embedder_slot(
    ctx: *mut JSContext,
    obj: JSValue,
    index: usize,
) -> Option<usize> {
    if unsafe { !qjs::JS_IsObject(obj) } || !object_is_raster_v8_class(ctx, obj) {
        return None;
    }
    let embedder = internal_field_ptr_for_object(obj, index)?;
    let base = unsafe { qjs::JS_VALUE_GET_PTR(obj) as *mut u8 };
    let slot = unsafe {
        base.add(V8_EMBEDDER_SLOT_OFFSET) as *mut usize
    };
    let saved = unsafe { *slot };
    unsafe { *slot = embedder };
    Some(saved)
}

pub(crate) unsafe fn restore_js_object_embedder_slot(obj: JSValue, saved: usize) {
    if unsafe { !qjs::JS_IsObject(obj) } {
        return;
    }
    let base = unsafe { qjs::JS_VALUE_GET_PTR(obj) as *mut u8 };
    let slot = unsafe {
        base.add(V8_EMBEDDER_SLOT_OFFSET) as *mut usize
    };
    unsafe { *slot = saved };
}

fn root_id_for_ptr(state: &crate::bridge::BridgeState, ptr: usize) -> Option<u64> {
    state.roots.find_id_by_ptr(ptr)
}

pub unsafe extern "C" fn root_id_for_js_object(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    object_ptr: *mut c_void,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() || object_ptr.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let ptr = object_ptr as usize;
        let Some(id) = root_id_for_ptr(state, ptr) else {
            return RasterV8Status::Error;
        };
        *out = id;
        RasterV8Status::Ok
    })
}

pub(crate) fn ensure_v8_object_class(ctx: *mut JSContext) -> qjs::JSClassID {
    let rt = unsafe { qjs::JS_GetRuntime(ctx) };
    let rt_key = rt as usize;
    let mut map = V8_OBJECT_CLASS.lock();
    if let Some(&id) = map.get(&rt_key) {
        return id;
    }
    let mut class_id: qjs::JSClassID = 0;
    unsafe {
        qjs::JS_NewClassID(rt, &mut class_id);
        let def = qjs::JSClassDef {
            class_name: c"RasterV8Object".as_ptr(),
            finalizer: Some(v8_object_finalizer),
            gc_mark: None,
            call: None,
            exotic: ptr::null_mut(),
        };
        qjs::JS_NewClass(rt, class_id, &def);
    }
    map.insert(rt_key, class_id);
    class_id
}

unsafe extern "C" fn v8_object_finalizer(_rt: *mut qjs::JSRuntime, val: JSValue) {
    if let Some(key) = object_ptr_key(val) {
        INTERNAL_FIELDS.lock().remove(&key);
        OBJECT_FIELD_COUNTS.lock().remove(&key);
        if let Some(slot) = WEAK_CALLBACKS.lock().remove(&key) {
            if let Some(cb) = slot.callback {
                cb(slot.parameter as *const c_void, 0);
            }
        }
    }
}

pub fn new_v8_object(ctx: *mut JSContext) -> JSValue {
    let class_id = ensure_v8_object_class(ctx);
    unsafe { qjs::JS_NewObjectClass(ctx, class_id) }
}

fn value_from_root(ctx: *mut JSContext, root: u64) -> Option<JSValue> {
    with_state(|state| state.roots.get(root).map(|v| unsafe { qjs::JS_DupValue(ctx, v) }))
}

fn atom_from_key(ctx: *mut JSContext, key: JSValue) -> Option<qjs::JSAtom> {
    let atom = unsafe { qjs::JS_ValueToAtom(ctx, key) };
    if atom == 0 {
        None
    } else {
        Some(atom)
    }
}

pub unsafe extern "C" fn object_get(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    object_root: u64,
    key_root: u64,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(obj) = state.roots.get(object_root) else {
            return RasterV8Status::Error;
        };
        let Some(key) = state.roots.get(key_root) else {
            return RasterV8Status::Error;
        };
        let Some(atom) = atom_from_key(state.ctx_ptr(), key) else {
            return RasterV8Status::Error;
        };
        let val = qjs::JS_GetProperty(state.ctx_ptr(), obj, atom);
        qjs::JS_FreeAtom(state.ctx_ptr(), atom);
        if qjs::JS_IsException(val) {
            return RasterV8Status::Exception;
        }
        *out = state.roots.insert(state.ctx_ptr(), val);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn object_get_index(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    object_root: u64,
    index: u32,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(obj) = state.roots.get(object_root) else {
            return RasterV8Status::Error;
        };
        let val = qjs::JS_GetPropertyUint32(state.ctx_ptr(), obj, index);
        if qjs::JS_IsException(val) {
            return RasterV8Status::Exception;
        }
        *out = state.roots.insert(state.ctx_ptr(), val);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn object_set_index(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    object_root: u64,
    index: u32,
    value_root: u64,
) -> RasterV8Status {
    with_state(|state| {
        let Some(obj) = state.roots.get(object_root) else {
            return RasterV8Status::Error;
        };
        let Some(val) = state.roots.get(value_root) else {
            return RasterV8Status::Error;
        };
        let rc = qjs::JS_SetPropertyUint32(
            state.ctx_ptr(),
            obj,
            index,
            qjs::JS_DupValue(state.ctx_ptr(), val),
        );
        if rc < 0 {
            return RasterV8Status::Exception;
        }
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn object_define_own_property(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    object_root: u64,
    key_root: u64,
    value_root: u64,
    attr: c_int,
    out: *mut bool,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
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
        let Some(atom) = atom_from_key(state.ctx_ptr(), key) else {
            return RasterV8Status::Error;
        };
        let mut flags = qjs::JS_PROP_C_W_E as c_int;
        if attr & 1 == 0 {
            flags |= qjs::JS_PROP_WRITABLE as c_int;
        }
        if attr & 2 == 0 {
            flags |= qjs::JS_PROP_ENUMERABLE as c_int;
        }
        if attr & 4 == 0 {
            flags |= qjs::JS_PROP_CONFIGURABLE as c_int;
        }
        let rc = qjs::JS_DefinePropertyValue(
            state.ctx_ptr(),
            obj,
            atom,
            qjs::JS_DupValue(state.ctx_ptr(), val),
            flags,
        );
        qjs::JS_FreeAtom(state.ctx_ptr(), atom);
        if rc < 0 {
            return RasterV8Status::Exception;
        }
        *out = true;
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn object_has_own_property(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    object_root: u64,
    key_root: u64,
    out: *mut bool,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(obj) = state.roots.get(object_root) else {
            return RasterV8Status::Error;
        };
        let Some(key) = state.roots.get(key_root) else {
            return RasterV8Status::Error;
        };
        let Some(atom) = atom_from_key(state.ctx_ptr(), key) else {
            return RasterV8Status::Error;
        };
        let mut prop = qjs::JSPropertyDescriptor {
            flags: 0,
            value: qjs::JS_UNDEFINED,
            getter: qjs::JS_UNDEFINED,
            setter: qjs::JS_UNDEFINED,
        };
        let rc = qjs::JS_GetOwnProperty(state.ctx_ptr(), &mut prop, obj, atom);
        qjs::JS_FreeAtom(state.ctx_ptr(), atom);
        if rc < 0 {
            return RasterV8Status::Exception;
        }
        if rc > 0 {
            qjs::JS_FreeValue(state.ctx_ptr(), prop.value);
            if !qjs::JS_IsUndefined(prop.getter) {
                qjs::JS_FreeValue(state.ctx_ptr(), prop.getter);
            }
            if !qjs::JS_IsUndefined(prop.setter) {
                qjs::JS_FreeValue(state.ctx_ptr(), prop.setter);
            }
        }
        *out = rc > 0;
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn object_get_prototype(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    object_root: u64,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(obj) = state.roots.get(object_root) else {
            return RasterV8Status::Error;
        };
        let proto = qjs::JS_GetPrototype(state.ctx_ptr(), obj);
        if qjs::JS_IsException(proto) {
            return RasterV8Status::Exception;
        }
        *out = state.roots.insert(state.ctx_ptr(), proto);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn array_new(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    length: c_int,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let arr = qjs::JS_NewArray(state.ctx_ptr());
        if length > 0 {
            let len = new_int32(state.ctx_ptr(), length);
            qjs::JS_SetPropertyStr(state.ctx_ptr(), arr, c"length".as_ptr(), len);
        }
        *out = state.roots.insert(state.ctx_ptr(), arr);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn number_new(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    value: f64,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let num = qjs::JS_NewFloat64(value);
        *out = state.roots.insert(state.ctx_ptr(), num);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn bigint_new(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    value: i64,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let bi = qjs::JS_NewBigInt64(state.ctx_ptr(), value);
        *out = state.roots.insert(state.ctx_ptr(), bi);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn integer_new(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    value: c_int,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let num = new_int32(state.ctx_ptr(), value);
        *out = state.roots.insert(state.ctx_ptr(), num);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn string_new_latin1(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    data: *const u8,
    length: c_int,
    out: *mut u64,
) -> RasterV8Status {
    if data.is_null() || out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let bytes = if length < 0 {
            std::ffi::CStr::from_ptr(data as *const c_char)
                .to_bytes()
        } else {
            std::slice::from_raw_parts(data, length as usize)
        };
        let js = qjs::JS_NewStringLen(
            state.ctx_ptr(),
            bytes.as_ptr() as *const c_char,
            bytes.len() as u64,
        );
        *out = state.roots.insert(state.ctx_ptr(), js);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn string_to_utf8(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    value_root: u64,
    out_ptr: *mut *mut c_char,
    out_len: *mut usize,
) -> RasterV8Status {
    if out_ptr.is_null() || out_len.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(val) = state.roots.get(value_root) else {
            return RasterV8Status::Error;
        };
        let mut len: usize = 0;
        let ptr = qjs::JS_ToCStringLen(state.ctx_ptr(), &mut len, val);
        if ptr.is_null() {
            return RasterV8Status::Error;
        }
        *out_ptr = ptr as *mut c_char;
        *out_len = len;
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn string_free_utf8(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    ptr: *mut c_char,
) -> RasterV8Status {
    with_state(|state| {
        if !ptr.is_null() {
            qjs::JS_FreeCString(state.ctx_ptr(), ptr);
        }
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn function_call(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    func_root: u64,
    recv_root: u64,
    argc: c_int,
    args: *const u64,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(func) = state.roots.get(func_root) else {
            return RasterV8Status::Error;
        };
        let recv = if recv_root == 0 {
            qjs::JS_UNDEFINED
        } else {
            let Some(r) = state.roots.get(recv_root) else {
                return RasterV8Status::Error;
            };
            qjs::JS_DupValue(state.ctx_ptr(), r)
        };
        let mut argv: Vec<JSValue> = Vec::with_capacity(argc.max(0) as usize);
        if !args.is_null() && argc > 0 {
            let slice = std::slice::from_raw_parts(args, argc as usize);
            for &root in slice {
                let Some(arg) = state.roots.get(root) else {
                    return RasterV8Status::Error;
                };
                argv.push(qjs::JS_DupValue(state.ctx_ptr(), arg));
            }
        }
        let result = qjs::JS_Call(
            state.ctx_ptr(),
            func,
            recv,
            argc,
            argv.as_mut_ptr(),
        );
        for arg in argv {
            qjs::JS_FreeValue(state.ctx_ptr(), arg);
        }
        if recv_root != 0 {
            qjs::JS_FreeValue(state.ctx_ptr(), recv);
        }
        if qjs::JS_IsException(result) {
            return RasterV8Status::Exception;
        }
        *out = state.roots.insert(state.ctx_ptr(), result);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn throw_value(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    value_root: u64,
) -> RasterV8Status {
    with_state(|state| {
        let Some(val) = state.roots.get(value_root) else {
            return RasterV8Status::Error;
        };
        qjs::JS_Throw(state.ctx_ptr(), qjs::JS_DupValue(state.ctx_ptr(), val));
        RasterV8Status::Exception
    })
}

pub unsafe extern "C" fn new_exception(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    msg_root: u64,
    kind: c_int,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(msg) = state.roots.get(msg_root) else {
            return RasterV8Status::Error;
        };
        let err = match kind {
            1 => {
                let err = qjs::JS_NewError(state.ctx_ptr());
                qjs::JS_SetPropertyStr(
                    state.ctx_ptr(),
                    err,
                    c"name".as_ptr(),
                    qjs::JS_NewStringLen(state.ctx_ptr(), c"TypeError".as_ptr(), 9),
                );
                err
            }
            2 => {
                let err = qjs::JS_NewError(state.ctx_ptr());
                qjs::JS_SetPropertyStr(
                    state.ctx_ptr(),
                    err,
                    c"name".as_ptr(),
                    qjs::JS_NewStringLen(state.ctx_ptr(), c"RangeError".as_ptr(), 10),
                );
                err
            }
            _ => qjs::JS_NewError(state.ctx_ptr()),
        };
        qjs::JS_SetPropertyStr(
            state.ctx_ptr(),
            err,
            c"message".as_ptr(),
            qjs::JS_DupValue(state.ctx_ptr(), msg),
        );
        *out = state.roots.insert(state.ctx_ptr(), err);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn external_new(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    ptr: *mut c_void,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let obj = new_v8_object(state.ctx_ptr());
        if let Some(key) = object_ptr_key(obj) {
            INTERNAL_FIELDS.lock().entry(key).or_default().push(ptr as usize);
        }
        *out = state.roots.insert(state.ctx_ptr(), obj);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn object_reserve_internal_fields(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    object_root: u64,
    count: c_int,
) -> RasterV8Status {
    if count < 0 {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(obj) = state.roots.get(object_root) else {
            return RasterV8Status::Error;
        };
        let Some(key) = object_ptr_key(obj) else {
            return RasterV8Status::Error;
        };
        let count = count as usize;
        OBJECT_FIELD_COUNTS.lock().insert(key, count);
        let mut fields = INTERNAL_FIELDS.lock();
        let entry = fields.entry(key).or_default();
        if entry.len() < count {
            entry.resize(count, 0);
        }
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn object_internal_field_count(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    object_root: u64,
    out: *mut c_int,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let Some(obj) = state.roots.get(object_root) else {
            return RasterV8Status::Error;
        };
        let Some(key) = object_ptr_key(obj) else {
            *out = 0;
            return RasterV8Status::Ok;
        };
        let count = OBJECT_FIELD_COUNTS
            .lock()
            .get(&key)
            .copied()
            .unwrap_or_else(|| INTERNAL_FIELDS.lock().get(&key).map(|v| v.len()).unwrap_or(0));
        *out = count as c_int;
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn internal_field_set(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    object_root: u64,
    index: c_int,
    ptr: *mut c_void,
) -> RasterV8Status {
    with_state(|state| {
        let Some(obj) = state.roots.get(object_root) else {
            return RasterV8Status::Error;
        };
        let Some(key) = object_ptr_key(obj) else {
            return RasterV8Status::Error;
        };
        let mut map = INTERNAL_FIELDS.lock();
        let fields = map.entry(key).or_default();
        let idx = index as usize;
        if fields.len() <= idx {
            fields.resize(idx + 1, 0);
        }
        fields[idx] = ptr as usize;
        let mut by_root = INTERNAL_FIELDS_BY_ROOT.lock();
        let root_fields = by_root.entry(object_root).or_default();
        if root_fields.len() <= idx {
            root_fields.resize(idx + 1, 0);
        }
        root_fields[idx] = ptr as usize;
        if index == 0 {
            set_embedder_field0_in_frame(ptr as usize);
        }
        let class_id = ensure_v8_object_class(state.ctx_ptr());
        unsafe {
            qjs::JS_SetOpaque(obj, ptr);
            let _ = class_id;
        }
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn internal_field_get(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    object_root: u64,
    index: c_int,
    out: *mut *mut c_void,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let obj = state.roots.get(object_root);
        let key = obj.and_then(object_ptr_key);
        let mut ptr = {
            let map = INTERNAL_FIELDS.lock();
            key.and_then(|k| {
                map.get(&k)
                    .and_then(|fields| fields.get(index as usize).copied())
            })
            .or_else(|| internal_field_ptr_for_root(object_root, index as usize))
            .unwrap_or(0)
        };
        if ptr == 0 && index == 0 {
            if let Some(obj) = obj {
                let class_id = ensure_v8_object_class(state.ctx_ptr());
                ptr = unsafe { qjs::JS_GetOpaque(obj, class_id) as usize };
                if ptr == 0 && unsafe { qjs::JS_IsObject(obj) } {
                    let base = unsafe { qjs::JS_VALUE_GET_PTR(obj) as *mut u8 };
                    ptr = unsafe { *(base.add(V8_EMBEDDER_SLOT_OFFSET) as *const usize) };
                }
            }
        }
        unsafe {
            *out = ptr as *mut c_void;
        }
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn symbol_iterator(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let global = qjs::JS_GetGlobalObject(state.ctx_ptr());
        let symbol = qjs::JS_GetPropertyStr(state.ctx_ptr(), global, c"Symbol".as_ptr());
        qjs::JS_FreeValue(state.ctx_ptr(), global);
        if qjs::JS_IsException(symbol) {
            return RasterV8Status::Exception;
        }
        let iter = qjs::JS_GetPropertyStr(state.ctx_ptr(), symbol, c"iterator".as_ptr());
        qjs::JS_FreeValue(state.ctx_ptr(), symbol);
        if qjs::JS_IsException(iter) {
            return RasterV8Status::Exception;
        }
        *out = state.roots.insert(state.ctx_ptr(), iter);
        RasterV8Status::Ok
    })
}

extern "C" {
    fn raster_v8_context_root_id(ctx: *mut crate::bridge::RasterV8ContextState) -> u64;
}

pub unsafe extern "C" fn get_context_root(
    ctx: *mut crate::bridge::RasterV8ContextState,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    *out = raster_v8_context_root_id(ctx);
    RasterV8Status::Ok
}

pub unsafe extern "C" fn oddball_root(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    root_index: c_int,
    out: *mut u64,
) -> RasterV8Status {
    if out.is_null() {
        return RasterV8Status::Error;
    }
    with_state(|state| {
        let ctx = state.ctx_ptr();
        let root = match root_index {
            4 => state.roots.insert(ctx, qjs::JS_DupValue(ctx, qjs::JS_UNDEFINED)),
            5 => state.roots.insert(ctx, qjs::JS_DupValue(ctx, qjs::JS_UNDEFINED)),
            6 => state.roots.insert(ctx, qjs::JS_DupValue(ctx, qjs::JS_NULL)),
            7 => state.roots.insert(ctx, qjs::JS_DupValue(ctx, qjs::JS_TRUE)),
            8 => state.roots.insert(ctx, qjs::JS_DupValue(ctx, qjs::JS_FALSE)),
            9 => {
                let empty = qjs::JS_NewStringLen(ctx, b"".as_ptr() as *const c_char, 0);
                let root = state.roots.insert(ctx, empty);
                qjs::JS_FreeValue(ctx, empty);
                root
            }
            _ => return RasterV8Status::Error,
        };
        *out = root;
        crate::root::mark_immortal_root(root);
        RasterV8Status::Ok
    })
}

pub unsafe extern "C" fn get_creation_context(
    ctx: *mut crate::bridge::RasterV8ContextState,
    _object_root: u64,
    out: *mut u64,
) -> RasterV8Status {
    get_context_root(ctx, out)
}

pub unsafe extern "C" fn register_weak_callback(
    _ctx: *mut crate::bridge::RasterV8ContextState,
    object_root: u64,
    parameter: *mut c_void,
    callback: Option<unsafe extern "C" fn(*const c_void, c_int)>,
) -> RasterV8Status {
    with_state(|state| {
        let Some(obj) = state.roots.get(object_root) else {
            return RasterV8Status::Error;
        };
        let Some(key) = object_ptr_key(obj) else {
            return RasterV8Status::Error;
        };
        WEAK_CALLBACKS.lock().insert(
            key,
            WeakSlot {
                callback,
                parameter: parameter as usize,
            },
        );
        RasterV8Status::Ok
    })
}

pub fn clear_runtime_tables(rt: *mut qjs::JSRuntime) {
    let rt_key = rt as usize;
    V8_OBJECT_CLASS.lock().remove(&rt_key);
    INTERNAL_FIELDS.lock().clear();
    INTERNAL_FIELDS_BY_ROOT.lock().clear();
    OBJECT_FIELD_COUNTS.lock().clear();
    WEAK_CALLBACKS.lock().clear();
}

pub fn prepare_shutdown(ctx: *mut JSContext) {
    INTERNAL_FIELDS.lock().clear();
    INTERNAL_FIELDS_BY_ROOT.lock().clear();
    OBJECT_FIELD_COUNTS.lock().clear();
    WEAK_CALLBACKS.lock().clear();
    let rt = unsafe { qjs::JS_GetRuntime(ctx) };
    clear_runtime_tables(rt);
}
