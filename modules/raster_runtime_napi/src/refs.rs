// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::os::raw::c_void;
use std::sync::atomic::{AtomicU64, Ordering};

use rquickjs::qjs::{self, JSContext, JSValue};

use crate::types::{napi_env, napi_ref, napi_status, napi_value};

static REF_ID: AtomicU64 = AtomicU64::new(1);

pub struct NapiRef {
    pub id: u64,
    pub value: JSValue,
    pub refcount: u32,
}

pub struct RefTable {
    /// Raw pointers into heap `NapiRef` boxes (non-owning; freed only in `delete`).
    live: Vec<usize>,
}

impl RefTable {
    pub fn new() -> Self {
        Self { live: Vec::new() }
    }

    pub fn create(
        &mut self,
        ctx: *mut JSContext,
        value: JSValue,
        initial_refcount: u32,
    ) -> napi_ref {
        let duped = unsafe { qjs::JS_DupValue(ctx, value) };
        let id = REF_ID.fetch_add(1, Ordering::Relaxed);
        let nref = Box::new(NapiRef {
            id,
            value: duped,
            refcount: initial_refcount,
        });
        let ptr = Box::into_raw(nref);
        self.live.push(ptr as usize);
        ptr as napi_ref
    }

    pub unsafe fn get(&self, reference: napi_ref) -> Option<&NapiRef> {
        if reference.is_null() {
            return None;
        }
        let ptr = reference as usize;
        if !self.live.contains(&ptr) {
            return None;
        }
        Some(&*(reference as *const NapiRef))
    }

    pub unsafe fn get_mut(&mut self, reference: napi_ref) -> Option<&mut NapiRef> {
        if reference.is_null() {
            return None;
        }
        let ptr = reference as usize;
        if !self.live.contains(&ptr) {
            return None;
        }
        Some(&mut *(reference as *mut NapiRef))
    }

    pub fn release_all(&mut self, ctx: *mut JSContext) {
        for ptr in self.live.drain(..) {
            let nref = unsafe { Box::from_raw(ptr as *mut NapiRef) };
            unsafe {
                qjs::JS_FreeValue(ctx, nref.value);
            }
        }
    }

    pub unsafe fn delete(&mut self, ctx: *mut JSContext, reference: napi_ref) -> napi_status {
        if reference.is_null() {
            return napi_status::napi_invalid_arg;
        }
        let ptr = reference as usize;
        if let Some(pos) = self.live.iter().position(|&p| p == ptr) {
            self.live.remove(pos);
            let nref = Box::from_raw(reference as *mut NapiRef);
            unsafe {
                qjs::JS_FreeValue(ctx, nref.value);
            }
            napi_status::napi_ok
        } else {
            napi_status::napi_invalid_arg
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_reference(
    env: napi_env,
    value: napi_value,
    initial_refcount: u32,
    result: *mut napi_ref,
) -> napi_status {
    if env.is_null() || value.is_null() || result.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let env = unsafe { crate::env::Env::from_napi_env(env) };
    let js_val = match unsafe { crate::value::napi_to_value(env, value) } {
        Some(v) => v,
        None => return napi_status::napi_invalid_arg,
    };
    let reference = env.refs.create(env.ctx_ptr(), js_val, initial_refcount);
    unsafe {
        *result = reference;
    }
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_delete_reference(env: napi_env, reference: napi_ref) -> napi_status {
    if env.is_null() || reference.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let env = unsafe { crate::env::Env::from_napi_env(env) };
    env.refs.delete(env.ctx_ptr(), reference)
}

#[no_mangle]
pub unsafe extern "C" fn napi_reference_ref(
    env: napi_env,
    reference: napi_ref,
    result: *mut u32,
) -> napi_status {
    if env.is_null() || reference.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let env = unsafe { crate::env::Env::from_napi_env(env) };
    let nref = match unsafe { env.refs.get_mut(reference) } {
        Some(r) => r,
        None => return napi_status::napi_invalid_arg,
    };
    nref.refcount = nref.refcount.saturating_add(1);
    if !result.is_null() {
        unsafe {
            *result = nref.refcount;
        }
    }
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_reference_unref(
    env: napi_env,
    reference: napi_ref,
    result: *mut u32,
) -> napi_status {
    if env.is_null() || reference.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let env = unsafe { crate::env::Env::from_napi_env(env) };
    let nref = match unsafe { env.refs.get_mut(reference) } {
        Some(r) => r,
        None => return napi_status::napi_invalid_arg,
    };
    nref.refcount = nref.refcount.saturating_sub(1);
    if !result.is_null() {
        unsafe {
            *result = nref.refcount;
        }
    }
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_get_reference_value(
    env: napi_env,
    reference: napi_ref,
    result: *mut napi_value,
) -> napi_status {
    if env.is_null() || reference.is_null() || result.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let env = unsafe { crate::env::Env::from_napi_env(env) };
    let nref = match unsafe { env.refs.get(reference) } {
        Some(r) => r,
        None => return napi_status::napi_invalid_arg,
    };
    unsafe {
        *result = crate::value::value_to_napi_borrowed(env, nref.value);
    }
    napi_status::napi_ok
}
