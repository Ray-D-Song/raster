// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_void;
use std::ptr;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};

use libc::pthread_t;
use rquickjs::qjs::{self, JSContext, JSValue};

use crate::driver::DriverState;
use crate::error::LastError;
use crate::finalizers::FinalizerTable;
use crate::refs::RefTable;
use crate::scopes::ScopeStack;
use crate::types::{napi_env, napi_extended_error_info, napi_finalize, napi_status};

pub type EnvCleanupHook = unsafe extern "C" fn(*mut c_void);
use crate::wrap::WrapTable;

static ENV_ID: AtomicU64 = AtomicU64::new(1);

pub struct Env {
    pub id: u64,
    pub ctx: NonNull<JSContext>,
    pub js_pthread: pthread_t,
    pub scopes: ScopeStack,
    pub refs: RefTable,
    pub wraps: WrapTable,
    pub finalizers: FinalizerTable,
    pub driver: Option<std::sync::Arc<DriverState>>,
    pub last_error: LastError,
    pub last_error_info: napi_extended_error_info,
    pub last_error_message: Option<CString>,
    pub instance_data: HashMap<u32, *mut c_void>,
    pub instance_finalize: Option<(napi_finalize, *mut c_void)>,
    pub cleanup_hooks: Vec<(EnvCleanupHook, *mut c_void)>,
    pub external_memory: i64,
    pub external_class_acquired: bool,
}

impl Env {
    pub fn new(ctx: NonNull<JSContext>) -> Self {
        Self {
            id: ENV_ID.fetch_add(1, Ordering::Relaxed),
            ctx,
            js_pthread: unsafe { libc::pthread_self() },
            scopes: ScopeStack::new(),
            refs: RefTable::new(),
            wraps: WrapTable::new(),
            finalizers: FinalizerTable::new(),
            driver: None,
            last_error: LastError::default(),
            last_error_info: napi_extended_error_info {
                error_message: ptr::null(),
                engine_reserved: ptr::null_mut(),
                error_code: napi_status::napi_ok,
                engine_error_code: 0,
            },
            last_error_message: None,
            instance_data: HashMap::new(),
            instance_finalize: None,
            cleanup_hooks: Vec::new(),
            external_memory: 0,
            external_class_acquired: false,
        }
    }

    pub fn is_js_thread(&self) -> bool {
        unsafe { libc::pthread_equal(libc::pthread_self(), self.js_pthread) != 0 }
    }

    pub fn ctx_ptr(&self) -> *mut JSContext {
        self.ctx.as_ptr()
    }

    pub fn as_napi_env(&self) -> napi_env {
        self as *const Env as *mut c_void
    }

    pub unsafe fn from_napi_env(env: napi_env) -> &'static mut Env {
        unsafe { &mut *(env as *mut Env) }
    }

    pub fn clear_exception(&mut self) -> Option<JSValue> {
        let ctx = self.ctx_ptr();
        unsafe {
            if !qjs::JS_HasException(ctx) {
                return None;
            }
            let exc = qjs::JS_GetException(ctx);
            if qjs::JS_IsNull(exc) || qjs::JS_IsUndefined(exc) {
                qjs::JS_FreeValue(ctx, exc);
                return None;
            }
            Some(exc)
        }
    }

    pub fn set_pending_exception(&mut self, value: JSValue) {
        let ctx = self.ctx_ptr();
        unsafe {
            qjs::JS_Throw(ctx, value);
        }
    }

    pub fn status_from_throw(&mut self) -> napi_status {
        if self.has_pending_exception() {
            napi_status::napi_pending_exception
        } else {
            napi_status::napi_generic_failure
        }
    }

    pub fn has_pending_exception(&self) -> bool {
        let ctx = self.ctx_ptr();
        unsafe { qjs::JS_HasException(ctx) }
    }

    pub fn set_last_error(&mut self, status: napi_status, message: Option<&str>) {
        self.last_error.status = status;
        self.last_error.message = message.map(|m| CString::new(m).unwrap_or_default());
        self.last_error_message = self.last_error.message.clone();
        self.last_error_info.error_code = status;
        self.last_error_info.error_message = self
            .last_error_message
            .as_ref()
            .map(|m| m.as_ptr())
            .unwrap_or(ptr::null());
    }

    pub fn last_error_info_ptr(&self) -> *const napi_extended_error_info {
        &self.last_error_info
    }

    /// Release all handle-scope roots so GC objects can be collected on shutdown.
    pub fn close_all_scopes(&mut self) {
        self.scopes.close_all(self.ctx_ptr());
    }

    /// Run cleanup hooks and release all JS roots held by this env.
    pub fn dispose(&mut self) {
        crate::async_work::close_all_tsfn_for_env(self.as_napi_env());
        let ctx = self.ctx_ptr();
        for (hook, arg) in self.cleanup_hooks.drain(..) {
            unsafe { hook(arg) };
        }
        if let Some((finalize, hint)) = self.instance_finalize.take() {
            let data = self.instance_data.remove(&0).unwrap_or(std::ptr::null_mut());
            if let Some(cb) = finalize {
                unsafe { cb(self.as_napi_env(), data, hint) };
            }
        } else {
            self.instance_data.clear();
        }
        self.refs.release_all(ctx);
        self.close_all_scopes();
        let rt = unsafe { qjs::JS_GetRuntime(ctx) };
        unsafe {
            qjs::JS_RunGC(rt);
        }
        crate::gc_hook::drain_pending_finalizers(self);
        crate::gc_hook::run_all_remaining(self);
        crate::external::finalize_surviving_externals(self.as_napi_env());
        if self.external_class_acquired {
            crate::external::release_external_class_for_env(rt);
            self.external_class_acquired = false;
        }
        crate::driver::shutdown_driver(self);
        self.wraps.clear();
    }
}
