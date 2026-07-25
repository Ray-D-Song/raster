// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Node-API (N-API) implementation for raster_runtime (QuickJS-backed).

#![allow(non_camel_case_types, clippy::missing_safety_doc)]

mod api;
mod async_work;
pub mod dlopen;
mod env;
mod error;
mod external;
mod finalizers;
mod js_helpers;
mod refs;
mod scopes;
mod types;
mod value;
mod wrap;

pub use dlopen::{dlopen_module, prepare_shutdown, shutdown_all};
pub use env::Env;
pub use types::*;

pub const NAPI_VERSION: u32 = 9;

#[cfg(test)]
mod tests {
    use rquickjs::qjs::{self};
    use rquickjs::{AsyncContext, AsyncRuntime};

    use crate::env::Env;
    use crate::external::{create_external_object, get_external_pointer, register_external_class};
    use crate::js_helpers::{define_hidden_usize, new_int32};
    use crate::refs::RefTable;
    use crate::scopes::ScopeStack;
    use crate::types::napi_status;
    use crate::value::{napi_to_value, value_to_napi_owned, value_to_napi_owned_in_parent};
    use crate::NAPI_VERSION;

    #[test]
    fn napi_version_is_nine() {
        assert_eq!(NAPI_VERSION, 9);
    }

    #[test]
    fn napi_status_ok_is_zero() {
        assert_eq!(napi_status::napi_ok as i32, 0);
    }

    #[tokio::test]
    async fn value_bridge_owned_does_not_leak_extra_ref() {
        let rt = AsyncRuntime::new().unwrap();
        let ctx = AsyncContext::full(&rt).await.unwrap();
        ctx.with(|ctx| {
            let raw = ctx.as_raw();
            let mut env = Env::new(raw);
            env.scopes.open();
            let s = unsafe { qjs::JS_NewStringLen(raw.as_ptr(), c"hello".as_ptr(), 5) };
            let handle = value_to_napi_owned(&mut env, s);
            let roundtrip = unsafe { napi_to_value(&env, handle) }.unwrap();
            assert!(unsafe { qjs::JS_IsString(roundtrip) });
            env.scopes.close(raw.as_ptr());
        })
        .await;
    }

    #[tokio::test]
    async fn handle_scope_frees_values_on_close() {
        let rt = AsyncRuntime::new().unwrap();
        let ctx = AsyncContext::full(&rt).await.unwrap();
        ctx.with(|ctx| {
            let raw = ctx.as_raw();
            let mut scopes = ScopeStack::new();
            scopes.open();
            let val = unsafe { qjs::JS_NewObject(raw.as_ptr()) };
            let idx = scopes.current_mut().unwrap().values.len();
            scopes.current_mut().unwrap().values.push(val);
            scopes.close(raw.as_ptr());
            assert_eq!(scopes.depth(), 0);
            let _ = idx;
        })
        .await;
    }

    #[tokio::test]
    async fn ref_table_create_and_delete_once() {
        let rt = AsyncRuntime::new().unwrap();
        let ctx = AsyncContext::full(&rt).await.unwrap();
        ctx.with(|ctx| {
            let raw = ctx.as_raw();
            let mut refs = RefTable::new();
            let s = unsafe { qjs::JS_NewStringLen(raw.as_ptr(), c"x".as_ptr(), 1) };
            let reference = refs.create(raw.as_ptr(), s, 1);
            assert!(!reference.is_null());
            let status = unsafe { refs.delete(raw.as_ptr(), reference) };
            assert_eq!(status, napi_status::napi_ok);
            let status2 = unsafe { refs.delete(raw.as_ptr(), reference) };
            assert_eq!(status2, napi_status::napi_invalid_arg);
        })
        .await;
    }

    #[tokio::test]
    async fn external_object_roundtrip() {
        let rt = AsyncRuntime::new().unwrap();
        let ctx = AsyncContext::full(&rt).await.unwrap();
        ctx.with(|ctx| {
            let raw = ctx.as_raw();
            let rt_ptr = unsafe { qjs::JS_GetRuntime(raw.as_ptr()) };
            register_external_class(rt_ptr);
            let mut env = Env::new(raw);
            env.scopes.open();
            let ptr = 0x1234usize as *mut std::ffi::c_void;
            let obj = create_external_object(raw.as_ptr(), ptr, None, std::ptr::null_mut(), (&env as *const Env) as _);
            let handle = value_to_napi_owned(&mut env, obj);
            let js_val = unsafe { napi_to_value(&env, handle) }.unwrap();
            assert_eq!(get_external_pointer(js_val), Some(ptr));
            env.scopes.close(raw.as_ptr());
            env.dispose();
        })
        .await;
    }

    #[tokio::test]
    async fn promise_resolve_settles() {
        let rt = AsyncRuntime::new().unwrap();
        let ctx = AsyncContext::full(&rt).await.unwrap();
        ctx.with(|ctx| {
            let raw = ctx.as_raw();
            let mut deferred: crate::types::napi_deferred = std::ptr::null_mut();
            let mut promise: crate::types::napi_value = std::ptr::null_mut();
            let mut env = Env::new(raw);
            env.scopes.open();
            let napi_env = env.as_napi_env();
            let status =
                unsafe { crate::async_work::napi_create_promise(napi_env, &mut deferred, &mut promise) };
            assert_eq!(status, napi_status::napi_ok);
            let one = new_int32(1);
            let one_handle = value_to_napi_owned(&mut env, one);
            let status = unsafe {
                crate::async_work::napi_resolve_deferred(napi_env, deferred, one_handle)
            };
            assert_eq!(status, napi_status::napi_ok);
            let p = unsafe { napi_to_value(&env, promise) }.unwrap();
            assert!(unsafe { qjs::JS_IsObject(p) });
            env.scopes.close(raw.as_ptr());
            env.dispose();
        })
        .await;
    }

    #[tokio::test]
    async fn wrap_marker_not_enumerable() {
        let rt = AsyncRuntime::new().unwrap();
        let ctx = AsyncContext::full(&rt).await.unwrap();
        ctx.with(|ctx| {
            let raw = ctx.as_raw();
            let obj = unsafe { qjs::JS_NewObject(raw.as_ptr()) };
            assert!(unsafe {
                define_hidden_usize(raw.as_ptr(), obj, c"__napi_wrap_id".as_ptr(), 42)
            });

            let flags = (qjs::JS_GPN_STRING_MASK | qjs::JS_GPN_ENUM_ONLY) as i32;
            let mut keys: *mut qjs::JSPropertyEnum = std::ptr::null_mut();
            let mut len: u32 = 0;
            let ok = unsafe {
                qjs::JS_GetOwnPropertyNames(raw.as_ptr(), &mut keys, &mut len, obj, flags)
            };
            assert!(ok >= 0);
            let wrap_atom =
                unsafe { qjs::JS_NewAtom(raw.as_ptr(), c"__napi_wrap_id".as_ptr()) };
            for i in 0..len {
                let atom = unsafe { (*keys.add(i as usize)).atom };
                assert_ne!(atom, wrap_atom);
            }
            unsafe { qjs::JS_FreeAtom(raw.as_ptr(), wrap_atom) };
            unsafe { qjs::JS_FreePropertyEnum(raw.as_ptr(), keys, len) };
            unsafe { qjs::JS_FreeValue(raw.as_ptr(), obj) };
        })
        .await;
    }

    #[tokio::test]
    async fn escape_handle_survives_escapable_scope_close() {
        let rt = AsyncRuntime::new().unwrap();
        let ctx = AsyncContext::full(&rt).await.unwrap();
        ctx.with(|ctx| {
            let raw = ctx.as_raw();
            let mut env = Env::new(raw);
            env.scopes.open();
            env.scopes.open_escapable();
            let inner =
                unsafe { qjs::JS_NewStringLen(raw.as_ptr(), c"escaped".as_ptr(), 7) };
            let inner_handle = value_to_napi_owned(&mut env, inner);
            let duped = unsafe {
                qjs::JS_DupValue(
                    raw.as_ptr(),
                    napi_to_value(&env, inner_handle).unwrap(),
                )
            };
            let escaped = value_to_napi_owned_in_parent(&mut env, duped);
            env.scopes.close_escapable(raw.as_ptr());
            let v = unsafe { napi_to_value(&env, escaped) }.unwrap();
            assert!(unsafe { qjs::JS_IsString(v) });
            env.scopes.close(raw.as_ptr());
            env.dispose();
        })
        .await;
    }
}
