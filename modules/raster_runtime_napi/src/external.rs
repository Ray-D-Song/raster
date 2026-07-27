// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::os::raw::c_void;
use std::ptr;

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rquickjs::qjs::{self, JSClassDef, JSClassID, JSContext, JSRuntime, JSValue};

use crate::gc_hook;
use crate::types::{napi_env, napi_finalize};

static EXTERNAL_CLASS_IDS: Lazy<Mutex<HashMap<usize, JSClassID>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static EXTERNAL_CLASS_ENV_REFS: Lazy<Mutex<HashMap<usize, usize>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Opaque pointers for external objects not yet collected by GC.
static LIVING_EXTERNALS: Lazy<Mutex<HashMap<usize, Vec<usize>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub struct ExternalData {
    pub ptr: *mut c_void,
    pub finalize: napi_finalize,
    pub hint: *mut c_void,
    pub env: crate::types::napi_env,
    pub addon_finalized: bool,
}

fn register_living_external(env: napi_env, opaque: *mut ExternalData) {
    LIVING_EXTERNALS
        .lock()
        .entry(env as usize)
        .or_default()
        .push(opaque as usize);
}

fn unregister_living_external(opaque: *mut ExternalData) {
    let key = opaque as usize;
    let env = unsafe { (*opaque).env as usize };
    if let Some(list) = LIVING_EXTERNALS.lock().get_mut(&env) {
        list.retain(|p| *p != key);
    }
}

/// Run finalize callbacks for external objects still tracked at env teardown.
///
/// Marks surviving externals as `addon_finalized` tombstones. A late QuickJS
/// class finalizer may still reclaim the opaque `Box` after class mapping is
/// gone; it must not re-run the addon finalize callback.
pub fn finalize_surviving_externals(env: napi_env) {
    let opaques: Vec<usize> = LIVING_EXTERNALS
        .lock()
        .remove(&(env as usize))
        .unwrap_or_default();
    for opaque_key in opaques {
        let opaque = opaque_key as *mut ExternalData;
        let data = unsafe { &mut *opaque };
        if !data.addon_finalized {
            if let Some(f) = data.finalize {
                unsafe {
                    f(data.env, data.ptr, data.hint);
                }
            }
            data.addon_finalized = true;
            data.finalize = None;
            data.ptr = std::ptr::null_mut();
        }
    }
}

#[cfg(test)]
pub fn reset_for_tests() {
    LIVING_EXTERNALS.lock().clear();
    EXTERNAL_CLASS_IDS.lock().clear();
    EXTERNAL_CLASS_ENV_REFS.lock().clear();
}

fn is_registered_external_class(class_id: JSClassID) -> bool {
    EXTERNAL_CLASS_IDS
        .lock()
        .values()
        .any(|&registered| registered == class_id)
}

unsafe extern "C" fn external_class_finalizer(_rt: *mut JSRuntime, val: JSValue) {
    let mut class_id: JSClassID = 0;
    let opaque = unsafe { qjs::JS_GetAnyOpaque(val, &mut class_id) };
    if opaque.is_null() {
        return;
    }
    let data_ptr = opaque as *mut ExternalData;
    unregister_living_external(data_ptr);
    let data = unsafe { Box::from_raw(data_ptr) };
    if data.addon_finalized {
        return;
    }
    let entry_id = gc_hook::register_gc_entry(
        gc_hook::GcEntryKind::External,
        data.ptr,
        data.finalize,
        data.hint,
        data.env,
        None,
    );
    gc_hook::enqueue_gc_entry(entry_id);
}

pub fn acquire_external_class_for_env(rt: *mut JSRuntime) -> JSClassID {
    let rt_key = rt as usize;
    {
        let mut refs = EXTERNAL_CLASS_ENV_REFS.lock();
        *refs.entry(rt_key).or_insert(0) += 1;
    }
    ensure_external_class_registered(rt)
}

pub fn release_external_class_for_env(rt: *mut JSRuntime) {
    let rt_key = rt as usize;
    let should_unregister = {
        let mut refs = EXTERNAL_CLASS_ENV_REFS.lock();
        let count = refs.get_mut(&rt_key);
        match count {
            None => false,
            Some(0) => false,
            Some(n) => {
                *n -= 1;
                if *n == 0 {
                    refs.remove(&rt_key);
                    true
                } else {
                    false
                }
            }
        }
    };
    if should_unregister {
        unregister_external_class(rt);
    }
}

fn ensure_external_class_registered(rt: *mut JSRuntime) -> JSClassID {
    let rt_key = rt as usize;
    if let Some(&class_id) = EXTERNAL_CLASS_IDS.lock().get(&rt_key) {
        return class_id;
    }
    let mut class_id: JSClassID = 0;
    unsafe {
        qjs::JS_NewClassID(rt, &mut class_id);
        let def = JSClassDef {
            class_name: c"NapiExternal".as_ptr(),
            finalizer: Some(external_class_finalizer),
            gc_mark: None,
            call: None,
            exotic: ptr::null_mut(),
        };
        qjs::JS_NewClass(rt, class_id, &def);
    }
    EXTERNAL_CLASS_IDS.lock().insert(rt_key, class_id);
    class_id
}

pub fn unregister_external_class(rt: *mut JSRuntime) {
    EXTERNAL_CLASS_IDS.lock().remove(&(rt as usize));
}

pub fn create_external_object(
    ctx: *mut JSContext,
    data: *mut c_void,
    finalize: napi_finalize,
    hint: *mut c_void,
    env: crate::types::napi_env,
) -> JSValue {
    let rt = unsafe { qjs::JS_GetRuntime(ctx) };
    let class_id = ensure_external_class_registered(rt);
    let obj = unsafe { qjs::JS_NewObjectClass(ctx, class_id) };
    let boxed = Box::new(ExternalData {
        ptr: data,
        finalize,
        hint,
        env,
        addon_finalized: false,
    });
    let opaque = Box::into_raw(boxed);
    register_living_external(env, opaque);
    unsafe {
        qjs::JS_SetOpaque(obj, opaque as *mut c_void);
    }
    obj
}

fn external_opaque(val: JSValue) -> Option<*mut ExternalData> {
    if !unsafe { qjs::JS_IsObject(val) } {
        return None;
    }
    let mut class_id: JSClassID = 0;
    let opaque = unsafe { qjs::JS_GetAnyOpaque(val, &mut class_id) };
    if opaque.is_null() {
        return None;
    }
    let data = opaque as *mut ExternalData;
    if is_registered_external_class(class_id) {
        Some(data)
    } else {
        None
    }
}

pub fn get_external_pointer(val: JSValue) -> Option<*mut c_void> {
    let data = external_opaque(val)?;
    Some(unsafe { (*data).ptr })
}

pub fn is_external_object(val: JSValue) -> bool {
    external_opaque(val).is_some()
}
