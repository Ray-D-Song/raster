// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::os::raw::c_char;
use std::sync::atomic::{AtomicU64, Ordering};

use rquickjs::qjs::{self, JSContext, JSValue};

use crate::env::Env;
use crate::types::{napi_env, napi_status, napi_value, NAPI_AUTO_LENGTH};

static VALUE_ID: AtomicU64 = AtomicU64::new(1);

#[repr(C)]
pub struct NapiValue {
    pub id: u64,
    pub value_index: usize,
    /// When true, `value_index` refers to the parent handle scope (used after escape).
    pub in_parent_scope: bool,
}

/// Transfer ownership of `value` into the current handle scope and return a handle.
pub fn value_to_napi_owned(env: &mut Env, value: JSValue) -> napi_value {
    value_to_napi_owned_in_scope(env, value, false)
}

/// Root `value` in the parent handle scope while an escapable scope is open.
pub fn value_to_napi_owned_in_parent(env: &mut Env, value: JSValue) -> napi_value {
    value_to_napi_owned_in_scope(env, value, true)
}

fn value_to_napi_owned_in_scope(env: &mut Env, value: JSValue, in_parent_scope: bool) -> napi_value {
    let value_index = if in_parent_scope {
        env.scopes.push_to_parent_handle_scope(value)
    } else if let Some(esc) = env.scopes.current_escapable_mut() {
        let idx = esc.values.len();
        esc.values.push(value);
        idx
    } else if let Some(scope) = env.scopes.current_mut() {
        let idx = scope.values.len();
        scope.values.push(value);
        idx
    } else {
        env.scopes.open();
        let scope = env.scopes.current_mut().unwrap();
        let idx = scope.values.len();
        scope.values.push(value);
        idx
    };

    let boxed = Box::new(NapiValue {
        id: VALUE_ID.fetch_add(1, Ordering::Relaxed),
        value_index,
        in_parent_scope,
    });
    let ptr = Box::into_raw(boxed);

    if in_parent_scope {
        env.scopes.push_handle_to_parent(ptr);
    } else if let Some(esc) = env.scopes.current_escapable_mut() {
        esc.handles.push(ptr);
    } else if let Some(scope) = env.scopes.current_mut() {
        scope.handles.push(ptr);
    }

    ptr as napi_value
}

/// Dup a borrowed `value` into the current handle scope.
pub fn value_to_napi_borrowed(env: &mut Env, value: JSValue) -> napi_value {
    let ctx = env.ctx_ptr();
    let duped = unsafe { qjs::JS_DupValue(ctx, value) };
    value_to_napi_owned(env, duped)
}

/// Dup a borrowed value into the current handle scope (alias for clarity at call sites).
pub fn value_to_napi(env: &mut Env, value: JSValue) -> napi_value {
    value_to_napi_borrowed(env, value)
}

pub unsafe fn napi_to_value(env: &Env, napi_val: napi_value) -> Option<JSValue> {
    if napi_val.is_null() {
        return None;
    }
    let nv = unsafe { &*(napi_val as *const NapiValue) };
    env.scopes
        .resolve_value(nv.value_index, nv.in_parent_scope)
}

pub unsafe fn napi_to_value_dup(env: &Env, napi_val: napi_value) -> Option<JSValue> {
    let value = unsafe { napi_to_value(env, napi_val)? };
    Some(unsafe { qjs::JS_DupValue(env.ctx_ptr(), value) })
}

pub unsafe fn free_napi_value(env: &Env, napi_val: napi_value) {
    if napi_val.is_null() {
        return;
    }
    let boxed = napi_val as *mut NapiValue;
    unsafe {
        let _ = Box::from_raw(boxed);
    }
    let _ = env;
}

pub fn bytes_from_js(
    ctx: *mut JSContext,
    value: JSValue,
    buf: *mut c_char,
    bufsize: usize,
    result: *mut usize,
    latin1: bool,
) -> napi_status {
    unsafe {
        if !qjs::JS_IsString(value) {
            return napi_status::napi_string_expected;
        }
        let mut len: usize = 0;
        let ptr = qjs::JS_ToCStringLen(ctx, &mut len as *mut _, value);
        if ptr.is_null() {
            return napi_status::napi_generic_failure;
        }
        let utf8 = std::slice::from_raw_parts(ptr as *const u8, len);
        let latin1_bytes: Vec<u8> = if latin1 {
            let s = std::str::from_utf8(utf8).unwrap_or("");
            s.chars()
                .map(|c| {
                    if c <= '\u{00FF}' {
                        c as u8
                    } else {
                        b'?'
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        let copy_len = if latin1 {
            latin1_bytes.len()
        } else {
            len
        };
        if !buf.is_null() && bufsize > 0 {
            let n = copy_len.min(bufsize.saturating_sub(1));
            if latin1 {
                std::ptr::copy_nonoverlapping(latin1_bytes.as_ptr(), buf as *mut u8, n);
            } else {
                std::ptr::copy_nonoverlapping(ptr as *const u8, buf as *mut u8, n);
            }
            *buf.add(n) = 0;
        }
        if !result.is_null() {
            *result = copy_len;
        }
        qjs::JS_FreeCString(ctx, ptr);
        napi_status::napi_ok
    }
}

pub fn string_from_bytes(
    env: &mut Env,
    data: *const u8,
    length: usize,
    latin1: bool,
) -> Result<napi_value, napi_status> {
    if data.is_null() && length > 0 {
        env.set_last_error(napi_status::napi_invalid_arg, Some("Null string data"));
        return Err(napi_status::napi_invalid_arg);
    }
    let slice = if length == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, length) }
    };
    let ctx = env.ctx_ptr();
    let js_str = if latin1 {
        unsafe { qjs::JS_NewStringLen(ctx, data as *const _, length as u64) }
    } else {
        match std::str::from_utf8(slice) {
            Ok(_) => unsafe { qjs::JS_NewStringLen(ctx, data as *const _, length as u64) },
            Err(_) => {
                env.set_last_error(napi_status::napi_invalid_arg, Some("Invalid UTF-8"));
                return Err(napi_status::napi_invalid_arg);
            }
        }
    };
    Ok(value_to_napi_owned(env, js_str))
}

pub fn string_from_cstr(
    env: &mut Env,
    data: *const std::os::raw::c_char,
    length: usize,
) -> Result<napi_value, napi_status> {
    if data.is_null() {
        env.set_last_error(napi_status::napi_invalid_arg, Some("Null string data"));
        return Err(napi_status::napi_invalid_arg);
    }
    let len = if length == NAPI_AUTO_LENGTH {
        unsafe { libc::strlen(data) }
    } else {
        length
    };
    string_from_bytes(env, data as *const u8, len, false)
}

pub fn cstr_from_js(
    ctx: *mut JSContext,
    value: JSValue,
    buf: *mut c_char,
    bufsize: usize,
    result: *mut usize,
) -> napi_status {
    bytes_from_js(ctx, value, buf, bufsize, result, false)
}
