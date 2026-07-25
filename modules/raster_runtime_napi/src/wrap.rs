// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::os::raw::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use rquickjs::qjs::{self, JSContext, JSValue};

use crate::js_helpers::{define_hidden_usize, read_hidden_usize};
use crate::types::{napi_finalize, napi_status};

static NEXT_WRAP_ID: AtomicUsize = AtomicUsize::new(1);

pub struct WrapEntry {
    pub native: *mut c_void,
    pub finalize: napi_finalize,
    pub finalize_hint: *mut c_void,
}

pub struct WrapTable {
    by_id: HashMap<usize, WrapEntry>,
}

impl WrapTable {
    pub fn new() -> Self {
        Self {
            by_id: HashMap::new(),
        }
    }

    unsafe fn clear_wrap_id(ctx: *mut JSContext, obj: JSValue) {
        let atom = qjs::JS_NewAtom(ctx, c"__napi_wrap_id".as_ptr());
        qjs::JS_DeleteProperty(ctx, obj, atom, 0);
        qjs::JS_FreeAtom(ctx, atom);
    }

    pub fn set(
        &mut self,
        ctx: *mut JSContext,
        obj: JSValue,
        native: *mut c_void,
        finalize: napi_finalize,
        hint: *mut c_void,
    ) -> bool {
        let id = NEXT_WRAP_ID.fetch_add(1, Ordering::Relaxed);
        if !unsafe { define_hidden_usize(ctx, obj, c"__napi_wrap_id".as_ptr(), id) } {
            return false;
        }
        self.by_id.insert(
            id,
            WrapEntry {
                native,
                finalize,
                finalize_hint: hint,
            },
        );
        true
    }

    pub fn get(&self, ctx: *mut JSContext, obj: JSValue) -> Option<*mut c_void> {
        let id = unsafe { read_hidden_usize(ctx, obj, c"__napi_wrap_id".as_ptr())? };
        self.by_id.get(&id).map(|e| e.native)
    }

    pub fn remove(&mut self, ctx: *mut JSContext, obj: JSValue) -> Option<WrapEntry> {
        let id = unsafe { read_hidden_usize(ctx, obj, c"__napi_wrap_id".as_ptr())? };
        unsafe { Self::clear_wrap_id(ctx, obj) };
        self.by_id.remove(&id)
    }

    pub fn clear(&mut self) {
        self.by_id.clear();
    }

    pub fn run_all_finalizers(&mut self, env: crate::types::napi_env) {
        for (_, entry) in self.by_id.drain() {
            if let Some(f) = entry.finalize {
                unsafe { f(env, entry.native, entry.finalize_hint) };
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn napi_wrap(
    env: crate::types::napi_env,
    js_object: crate::types::napi_value,
    native_object: *mut c_void,
    finalize_cb: napi_finalize,
    finalize_hint: *mut c_void,
    result: *mut crate::types::napi_ref,
) -> napi_status {
    if env.is_null() || js_object.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let env = unsafe { crate::env::Env::from_napi_env(env) };
    let ctx = env.ctx_ptr();
    let obj = match unsafe { crate::value::napi_to_value(env, js_object) } {
        Some(v) => v,
        None => return napi_status::napi_invalid_arg,
    };
    if !unsafe { qjs::JS_IsObject(obj) } {
        return napi_status::napi_object_expected;
    }
    if env.wraps.get(ctx, obj).is_some() {
        return napi_status::napi_invalid_arg;
    }
    if !env
        .wraps
        .set(ctx, obj, native_object, finalize_cb, finalize_hint)
    {
        return napi_status::napi_generic_failure;
    }
    if !result.is_null() {
        let status = unsafe {
            crate::refs::napi_create_reference(env.as_napi_env(), js_object, 0, result)
        };
        if status != napi_status::napi_ok {
            return status;
        }
    }
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_unwrap(
    env: crate::types::napi_env,
    js_object: crate::types::napi_value,
    result: *mut *mut c_void,
) -> napi_status {
    if env.is_null() || js_object.is_null() || result.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let env = unsafe { crate::env::Env::from_napi_env(env) };
    let ctx = env.ctx_ptr();
    let obj = match unsafe { crate::value::napi_to_value(env, js_object) } {
        Some(v) => v,
        None => return napi_status::napi_invalid_arg,
    };
    match env.wraps.get(ctx, obj) {
        Some(native) => {
            unsafe {
                *result = native;
            }
            napi_status::napi_ok
        }
        None => napi_status::napi_generic_failure,
    }
}

#[no_mangle]
pub unsafe extern "C" fn napi_remove_wrap(
    env: crate::types::napi_env,
    js_object: crate::types::napi_value,
    result: *mut *mut c_void,
) -> napi_status {
    if env.is_null() || js_object.is_null() || result.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let env = unsafe { crate::env::Env::from_napi_env(env) };
    let ctx = env.ctx_ptr();
    let obj = match unsafe { crate::value::napi_to_value(env, js_object) } {
        Some(v) => v,
        None => return napi_status::napi_invalid_arg,
    };
    match env.wraps.remove(ctx, obj) {
        Some(entry) => {
            unsafe {
                *result = entry.native;
            }
            if let Some(f) = entry.finalize {
                unsafe { f(env.as_napi_env(), entry.native, entry.finalize_hint) };
            }
            napi_status::napi_ok
        }
        None => napi_status::napi_generic_failure,
    }
}
