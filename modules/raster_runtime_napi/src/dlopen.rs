// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::ptr::{self, NonNull};
use std::sync::Mutex;

use libloading::Library;
use once_cell::sync::Lazy;
use rquickjs::qjs::{self, JSContext, JSValue};
use rquickjs::Ctx;
use tracing::debug;

use crate::api::take_pending_module;
use crate::env::Env;
use crate::types::{napi_addon_register_func, napi_env, napi_value};
use crate::value::value_to_napi_borrowed;

struct EnvPtr(*mut Env);
unsafe impl Send for EnvPtr {}
unsafe impl Sync for EnvPtr {}

static ENV_REGISTRY: Lazy<Mutex<HashMap<usize, EnvPtr>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

thread_local! {
    static CURRENT_ENV: RefCell<Option<*mut Env>> = const { RefCell::new(None) };
}

pub fn register_env(ctx: NonNull<JSContext>, env: Box<Env>) -> *mut Env {
    let ptr = Box::into_raw(env);
    let key = ctx.as_ptr() as usize;
    ENV_REGISTRY.lock().unwrap().insert(key, EnvPtr(ptr));
    ptr
}

#[allow(dead_code)]
pub fn unregister_env(ctx: NonNull<JSContext>) {
    let key = ctx.as_ptr() as usize;
    if let Some(EnvPtr(ptr)) = ENV_REGISTRY.lock().unwrap().remove(&key) {
        unsafe {
            (*ptr).dispose();
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Release all N-API env roots before the QuickJS runtime is torn down.
pub fn shutdown_all() {
    let mut registry = ENV_REGISTRY.lock().unwrap();
    for (_, EnvPtr(ptr)) in registry.drain() {
        unsafe {
            (*ptr).dispose();
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Clear require cache entries for native addons and dispose N-API env state.
/// Must run while the JS context is still alive (before `Vm` is dropped).
pub fn prepare_shutdown<'js>(ctx: &Ctx<'js>) {
    let remaining: usize = ctx
        .eval(
            r#"
            (() => {
                const req = globalThis.require;
                if (!req || !req.cache) return 0;
                const keys = Object.keys(req.cache);
                for (const key of keys) {
                    delete req.cache[key];
                }
                return keys.length;
            })()
            "#,
        )
        .unwrap_or(0);
    if remaining > 0 {
        tracing::debug!("napi prepare_shutdown: cleared {} require cache entries", remaining);
    }
    ctx.run_gc();
    crate::api::clear_function_callbacks();
    shutdown_all();
}

pub fn env_for_ctx(ctx: *mut JSContext) -> Option<*mut Env> {
    ENV_REGISTRY.lock().unwrap().get(&(ctx as usize)).map(|EnvPtr(p)| *p)
}

pub fn current_env() -> Option<&'static mut Env> {
    CURRENT_ENV.with(|cell| {
        let ptr = *cell.borrow();
        ptr.map(|p| unsafe { &mut *p })
    })
}

pub fn with_current_env_for_ctx<F, R>(ctx: *mut JSContext, f: F) -> Option<R>
where
    F: FnOnce(&mut Env) -> R,
{
    let ptr = env_for_ctx(ctx)?;
    CURRENT_ENV.with(|cell| {
        *cell.borrow_mut() = Some(ptr);
    });
    let result = f(unsafe { &mut *ptr });
    CURRENT_ENV.with(|cell| {
        *cell.borrow_mut() = None;
    });
    Some(result)
}

struct EnvActivation {
    _ptr: *mut Env,
}

impl EnvActivation {
    fn enter(ptr: *mut Env) -> Self {
        CURRENT_ENV.with(|cell| {
            *cell.borrow_mut() = Some(ptr);
        });
        Self { _ptr: ptr }
    }
}

impl Drop for EnvActivation {
    fn drop(&mut self) {
        CURRENT_ENV.with(|cell| {
            *cell.borrow_mut() = None;
        });
    }
}

/// Load a native addon and populate `module.exports`.
pub fn dlopen_module(
    ctx_ptr: NonNull<JSContext>,
    exports_obj: JSValue,
) -> Result<JSValue, String> {
    let filename = resolve_addon_path(exports_obj, ctx_ptr)?;
    debug!("dlopen: loading {}", filename);

    #[cfg(target_env = "musl")]
    {
        let _ = (ctx_ptr, exports_obj);
        return Err(
            "Native addon loading requires a dynamically linked raster_runtime build (not musl+static)"
                .to_string(),
        );
    }

    #[cfg(not(target_env = "musl"))]
    {
        dlopen_impl(ctx_ptr, exports_obj, &filename)?;
        Ok(exports_obj)
    }
}

fn resolve_addon_path(exports_obj: JSValue, ctx_ptr: NonNull<JSContext>) -> Result<String, String> {
    let _ = (exports_obj, ctx_ptr);
    Err("dlopen path resolution requires module context".to_string())
}

pub fn dlopen_file(
    ctx_ptr: NonNull<JSContext>,
    filename: &str,
) -> Result<JSValue, String> {
    debug!("dlopen_file: {}", filename);

    #[cfg(target_env = "musl")]
    {
        let _ = (ctx_ptr, filename);
        return Err(
            "Native addon loading requires a dynamically linked raster_runtime build (not musl+static)"
                .to_string(),
        );
    }

    #[cfg(not(target_env = "musl"))]
    {
        let exports = unsafe {
            let ctx = ctx_ptr.as_ptr();
            qjs::JS_NewObject(ctx)
        };
        dlopen_impl(ctx_ptr, exports, filename)?;
        Ok(exports)
    }
}

#[cfg(not(target_env = "musl"))]
fn dlopen_impl(
    ctx_ptr: NonNull<JSContext>,
    exports_obj: JSValue,
    filename: &str,
) -> Result<(), String> {
    if !Path::new(filename).exists() {
        return Err(format!("Cannot load addon '{}': file not found", filename));
    }

    let env_ptr = if let Some(existing) = env_for_ctx(ctx_ptr.as_ptr()) {
        existing
    } else {
        register_env(ctx_ptr, Box::new(Env::new(ctx_ptr)))
    };
    unsafe {
        let rt = qjs::JS_GetRuntime(ctx_ptr.as_ptr());
        crate::external::register_external_class(rt);
        (*env_ptr).scopes.open();
    }
    let _activation = EnvActivation::enter(env_ptr);

    let napi_env = unsafe { (*env_ptr).as_napi_env() };
    let exports_napi = value_to_napi_borrowed(unsafe { &mut *env_ptr }, exports_obj);

    let library = unsafe { Library::new(filename) }
        .map_err(|e| format!("dlopen failed for '{}': {}", filename, e))?;

    if let Some(module) = take_pending_module() {
        return run_register_func(
            napi_env,
            module.nm_register_func,
            exports_napi,
            ctx_ptr,
            exports_obj,
        );
    }

    type RegisterV1Fn = unsafe extern "C" fn(env: napi_env, exports: napi_value) -> napi_value;

    let register: libloading::Symbol<RegisterV1Fn> = unsafe {
        library.get(b"napi_register_module_v1\0").map_err(|_| {
            format!(
                "No napi_register_module_v1 symbol in '{}'. Is this an N-API addon?",
                filename
            )
        })?
    };

    let result = unsafe { register(napi_env, exports_napi) };
    if !result.is_null() {
        let js_result = unsafe { crate::value::napi_to_value_dup(&mut *env_ptr, result) }
            .ok_or_else(|| "Invalid return value from addon init".to_string())?;
        assign_module_exports(ctx_ptr.as_ptr(), exports_obj, js_result);
    }

    std::mem::forget(library);
    unsafe {
        (*env_ptr).close_all_scopes();
    }
    Ok(())
}

fn run_register_func(
    napi_env: napi_env,
    register: napi_addon_register_func,
    exports: napi_value,
    ctx_ptr: NonNull<JSContext>,
    exports_obj: JSValue,
) -> Result<(), String> {
    let result = unsafe { register(napi_env, exports) };
    if !result.is_null() {
        let env = unsafe { Env::from_napi_env(napi_env) };
        if let Some(js_result) = unsafe { crate::value::napi_to_value_dup(env, result) } {
            assign_module_exports(ctx_ptr.as_ptr(), exports_obj, js_result);
        }
    }
    Ok(())
}

/// Node replaces `module.exports` when the init function returns a distinct object.
fn assign_module_exports(ctx: *mut qjs::JSContext, exports_obj: JSValue, js_result: JSValue) {
    unsafe {
        if qjs::JS_IsStrictEqual(ctx, js_result, exports_obj) {
            qjs::JS_FreeValue(ctx, js_result);
            return;
        }

        let flags = (qjs::JS_GPN_STRING_MASK | qjs::JS_GPN_ENUM_ONLY) as i32;
        let mut keys: *mut qjs::JSPropertyEnum = ptr::null_mut();
        let mut len: u32 = 0;

        if qjs::JS_GetOwnPropertyNames(ctx, &mut keys, &mut len, exports_obj, flags) >= 0 {
            for i in 0..len {
                let atom = (*keys.add(i as usize)).atom;
                qjs::JS_DeleteProperty(ctx, exports_obj, atom, 0);
            }
            qjs::JS_FreePropertyEnum(ctx, keys, len);
        }

        keys = ptr::null_mut();
        len = 0;
        if qjs::JS_GetOwnPropertyNames(ctx, &mut keys, &mut len, js_result, flags) < 0 {
            qjs::JS_FreeValue(ctx, js_result);
            return;
        }

        for i in 0..len {
            let atom = (*keys.add(i as usize)).atom;
            let val = qjs::JS_GetProperty(ctx, js_result, atom);
            qjs::JS_SetProperty(ctx, exports_obj, atom, val);
        }
        qjs::JS_FreePropertyEnum(ctx, keys, len);
        qjs::JS_FreeValue(ctx, js_result);
    }
}

/// Rust entry for `process.dlopen(module, filename[, flags])`.
pub fn process_dlopen(
    ctx_ptr: NonNull<JSContext>,
    module_exports: JSValue,
    filename: &str,
    _flags: u32,
) -> Result<(), String> {
    dlopen_impl(ctx_ptr, module_exports, filename)
}
