// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::os::raw::c_void;
use std::ptr;

use parking_lot::Mutex;

use rquickjs::qjs::{self, JSValue};

use crate::env::Env;
use crate::types::*;

struct AsyncWork {
    execute: napi_async_execute_callback,
    complete: napi_async_complete_callback,
    data: *mut c_void,
    env: napi_env,
    cancelled: bool,
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_async_work(
    env: napi_env,
    _async_resource: napi_value,
    _async_resource_name: napi_value,
    execute: napi_async_execute_callback,
    complete: napi_async_complete_callback,
    data: *mut c_void,
    result: *mut napi_async_work,
) -> napi_status {
    if env.is_null() || result.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let work = Box::new(AsyncWork {
        execute,
        complete,
        data,
        env,
        cancelled: false,
    });
    let ptr = Box::into_raw(work);
    unsafe {
        *result = ptr as napi_async_work;
    }
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_delete_async_work(
    _env: napi_env,
    work: napi_async_work,
) -> napi_status {
    if work.is_null() {
        return napi_status::napi_invalid_arg;
    }
    unsafe {
        let _ = Box::from_raw(work as *mut AsyncWork);
    }
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_cancel_async_work(
    _env: napi_env,
    work: napi_async_work,
) -> napi_status {
    if work.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let w = unsafe { &mut *(work as *mut AsyncWork) };
    w.cancelled = true;
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_queue_async_work(_env: napi_env, work: napi_async_work) -> napi_status {
    if work.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let work_ref = unsafe { &mut *(work as *mut AsyncWork) };
    let execute = work_ref.execute;
    let complete = work_ref.complete;
    let data = work_ref.data;
    let env = work_ref.env;
    let cancelled = work_ref.cancelled;

    if !cancelled {
        if let Some(exec) = execute {
            let env_ptr = env as usize;
            let data_ptr = data as usize;
            let _ = std::thread::spawn(move || {
                exec(env_ptr as napi_env, data_ptr as *mut c_void);
            })
            .join();
        }
    }

    if let Some(comp) = complete {
        let status = if cancelled {
            napi_status::napi_cancelled
        } else {
            napi_status::napi_ok
        };
        comp(env, status, data);
    }

    napi_status::napi_ok
}

// Thread-safe functions

struct ThreadsafeFunction {
    env: napi_env,
    func_ref: napi_ref,
    call_js: napi_threadsafe_function_call_js,
    context: *mut c_void,
    queue: Mutex<Vec<*mut c_void>>,
    refs: u32,
}

struct TsfnPtr(*mut ThreadsafeFunction);
unsafe impl Send for TsfnPtr {}
unsafe impl Sync for TsfnPtr {}

static TSFN_LIST: Mutex<Vec<TsfnPtr>> = Mutex::new(Vec::new());

pub(crate) fn has_pending_tsfn() -> bool {
    !TSFN_LIST.lock().is_empty()
}

pub(crate) fn drain_threadsafe_functions(env: napi_env) {
    let list: Vec<*mut ThreadsafeFunction> =
        TSFN_LIST.lock().iter().map(|TsfnPtr(p)| *p).collect();
    for ptr in list {
        let tsfn = unsafe { &mut *ptr };
        if tsfn.env != env {
            continue;
        }
        let pending: Vec<*mut c_void> = tsfn.queue.lock().drain(..).collect();
        for data in pending {
            invoke_tsfn_call(tsfn, data);
        }
    }
}

fn invoke_tsfn_call(tsfn: &ThreadsafeFunction, data: *mut c_void) {
    let Some(call_js) = tsfn.call_js else {
        return;
    };
    let mut func_val: napi_value = ptr::null_mut();
    let status = unsafe {
        crate::refs::napi_get_reference_value(tsfn.env, tsfn.func_ref, &mut func_val)
    };
    if status != napi_status::napi_ok || func_val.is_null() {
        return;
    }
    unsafe {
        call_js(tsfn.env, func_val, tsfn.context, data);
    }
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_threadsafe_function(
    env: napi_env,
    func: napi_value,
    _async_resource: napi_value,
    _async_resource_name: napi_value,
    _max_queue_size: usize,
    initial_thread_count: usize,
    _thread_finalize_data: *mut c_void,
    _thread_finalize_cb: napi_finalize,
    context: *mut c_void,
    call_js_cb: napi_threadsafe_function_call_js,
    result: *mut napi_threadsafe_function,
) -> napi_status {
    if env.is_null() || func.is_null() || result.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let mut func_ref: napi_ref = ptr::null_mut();
    let status = unsafe { crate::refs::napi_create_reference(env, func, 1, &mut func_ref) };
    if status != napi_status::napi_ok {
        return status;
    }
    let tsfn = Box::new(ThreadsafeFunction {
        env,
        func_ref,
        call_js: call_js_cb,
        context,
        queue: Mutex::new(Vec::new()),
        refs: initial_thread_count.max(1) as u32,
    });
    let ptr = Box::into_raw(tsfn);
    TSFN_LIST.lock().push(TsfnPtr(ptr));
    unsafe {
        *result = ptr as napi_threadsafe_function;
    }
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_call_threadsafe_function(
    func: napi_threadsafe_function,
    data: *mut c_void,
    mode: napi_threadsafe_function_call_mode,
) -> napi_status {
    if func.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let tsfn = unsafe { &mut *(func as *mut ThreadsafeFunction) };
    if mode == napi_threadsafe_function_call_mode::napi_tsfn_blocking {
        invoke_tsfn_call(tsfn, data);
    } else {
        tsfn.queue.lock().push(data);
        drain_threadsafe_functions(tsfn.env);
    }
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_acquire_threadsafe_function(
    func: napi_threadsafe_function,
) -> napi_status {
    if func.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let tsfn = unsafe { &mut *(func as *mut ThreadsafeFunction) };
    tsfn.refs = tsfn.refs.saturating_add(1);
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_release_threadsafe_function(
    func: napi_threadsafe_function,
    mode: napi_threadsafe_function_release_mode,
) -> napi_status {
    if func.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let tsfn = unsafe { &mut *(func as *mut ThreadsafeFunction) };
    if mode == napi_threadsafe_function_release_mode::napi_tsfn_abort {
        tsfn.queue.lock().clear();
    }
    if tsfn.refs > 0 {
        tsfn.refs -= 1;
    }
    if tsfn.refs == 0 {
        TSFN_LIST.lock().retain(|TsfnPtr(p)| *p != func as *mut ThreadsafeFunction);
        unsafe {
            crate::refs::napi_delete_reference(tsfn.env, tsfn.func_ref);
            let _ = Box::from_raw(func as *mut ThreadsafeFunction);
        }
    }
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_unref_threadsafe_function(
    _env: napi_env,
    _func: napi_threadsafe_function,
) -> napi_status {
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_ref_threadsafe_function(
    _env: napi_env,
    _func: napi_threadsafe_function,
) -> napi_status {
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_get_threadsafe_function_context(
    func: napi_threadsafe_function,
    result: *mut *mut c_void,
) -> napi_status {
    if func.is_null() || result.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let tsfn = unsafe { &*(func as *const ThreadsafeFunction) };
    unsafe {
        *result = tsfn.context;
    }
    napi_status::napi_ok
}

// Promises

struct NapiDeferred {
    resolve_fn: JSValue,
    reject_fn: JSValue,
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_promise(
    env: napi_env,
    deferred: *mut napi_deferred,
    promise: *mut napi_value,
) -> napi_status {
    if env.is_null() || deferred.is_null() || promise.is_null() {
        return napi_status::napi_invalid_arg;
    }
    with_env_promise(env, deferred, promise)
}

fn with_env_promise(env: napi_env, deferred: *mut napi_deferred, promise: *mut napi_value) -> napi_status {
    let e = unsafe { Env::from_napi_env(env) };
    let ctx = e.ctx_ptr();
    let mut resolving: [JSValue; 2] = [qjs::JS_UNDEFINED, qjs::JS_UNDEFINED];
    let p = unsafe { qjs::JS_NewPromiseCapability(ctx, resolving.as_mut_ptr()) };
    if unsafe { qjs::JS_IsException(p) } {
        return napi_status::napi_generic_failure;
    }
    let boxed = Box::new(NapiDeferred {
        resolve_fn: resolving[0],
        reject_fn: resolving[1],
    });
    unsafe {
        *deferred = Box::into_raw(boxed) as napi_deferred;
        *promise = crate::value::value_to_napi_owned(e, p);
    }
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_resolve_deferred(
    env: napi_env,
    deferred: napi_deferred,
    resolution: napi_value,
) -> napi_status {
    if env.is_null() || deferred.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let e = unsafe { Env::from_napi_env(env) };
    let ctx = e.ctx_ptr();
    let def = unsafe { Box::from_raw(deferred as *mut NapiDeferred) };
    let mut val = if resolution.is_null() {
        qjs::JS_UNDEFINED
    } else {
        match unsafe { crate::value::napi_to_value_dup(e, resolution) } {
            Some(v) => v,
            None => {
                unsafe {
                    qjs::JS_FreeValue(ctx, def.resolve_fn);
                    qjs::JS_FreeValue(ctx, def.reject_fn);
                }
                return napi_status::napi_invalid_arg;
            }
        }
    };
    let result = unsafe { qjs::JS_Call(ctx, def.resolve_fn, qjs::JS_UNDEFINED, 1, &mut val) };
    if !resolution.is_null() {
        unsafe { qjs::JS_FreeValue(ctx, val) };
    }
    if unsafe { qjs::JS_IsException(result) } {
        unsafe {
            qjs::JS_FreeValue(ctx, def.resolve_fn);
            qjs::JS_FreeValue(ctx, def.reject_fn);
            qjs::JS_FreeValue(ctx, result);
        }
        return napi_status::napi_pending_exception;
    }
    unsafe {
        qjs::JS_FreeValue(ctx, result);
        qjs::JS_FreeValue(ctx, def.resolve_fn);
        qjs::JS_FreeValue(ctx, def.reject_fn);
    }
    napi_status::napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_reject_deferred(
    env: napi_env,
    deferred: napi_deferred,
    rejection: napi_value,
) -> napi_status {
    if env.is_null() || deferred.is_null() {
        return napi_status::napi_invalid_arg;
    }
    let e = unsafe { Env::from_napi_env(env) };
    let ctx = e.ctx_ptr();
    let def = unsafe { Box::from_raw(deferred as *mut NapiDeferred) };
    let mut val = if rejection.is_null() {
        qjs::JS_UNDEFINED
    } else {
        match unsafe { crate::value::napi_to_value_dup(e, rejection) } {
            Some(v) => v,
            None => {
                unsafe {
                    qjs::JS_FreeValue(ctx, def.resolve_fn);
                    qjs::JS_FreeValue(ctx, def.reject_fn);
                }
                return napi_status::napi_invalid_arg;
            }
        }
    };
    let result = unsafe { qjs::JS_Call(ctx, def.reject_fn, qjs::JS_UNDEFINED, 1, &mut val) };
    if !rejection.is_null() {
        unsafe { qjs::JS_FreeValue(ctx, val) };
    }
    if unsafe { qjs::JS_IsException(result) } {
        unsafe {
            qjs::JS_FreeValue(ctx, def.resolve_fn);
            qjs::JS_FreeValue(ctx, def.reject_fn);
            qjs::JS_FreeValue(ctx, result);
        }
        return napi_status::napi_pending_exception;
    }
    unsafe {
        qjs::JS_FreeValue(ctx, result);
        qjs::JS_FreeValue(ctx, def.resolve_fn);
        qjs::JS_FreeValue(ctx, def.reject_fn);
    }
    napi_status::napi_ok
}
