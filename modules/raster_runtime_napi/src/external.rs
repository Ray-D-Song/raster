// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::os::raw::c_void;
use std::ptr;
use std::sync::OnceLock;

use rquickjs::qjs::{self, JSClassDef, JSClassID, JSContext, JSRuntime, JSValue};

use crate::types::napi_finalize;

static EXTERNAL_CLASS_ID: OnceLock<JSClassID> = OnceLock::new();

pub struct ExternalData {
    pub ptr: *mut c_void,
    pub finalize: napi_finalize,
    pub hint: *mut c_void,
    pub env: crate::types::napi_env,
}

unsafe extern "C" fn external_class_finalizer(_rt: *mut JSRuntime, val: JSValue) {
    let class_id = external_class_id();
    let opaque = unsafe { qjs::JS_GetOpaque(val, class_id) };
    if opaque.is_null() {
        return;
    }
    let data = unsafe { Box::from_raw(opaque as *mut ExternalData) };
    if let Some(f) = data.finalize {
        unsafe { f(data.env, data.ptr, data.hint) };
    }
}

pub fn external_class_id() -> JSClassID {
    *EXTERNAL_CLASS_ID.get().expect("external class not registered")
}

pub fn register_external_class(rt: *mut JSRuntime) -> JSClassID {
    *EXTERNAL_CLASS_ID.get_or_init(|| {
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
        class_id
    })
}

pub fn create_external_object(
    ctx: *mut JSContext,
    data: *mut c_void,
    finalize: napi_finalize,
    hint: *mut c_void,
    env: crate::types::napi_env,
) -> JSValue {
    let rt = unsafe { qjs::JS_GetRuntime(ctx) };
    let class_id = register_external_class(rt);
    let obj = unsafe { qjs::JS_NewObjectClass(ctx, class_id) };
    let boxed = Box::new(ExternalData {
        ptr: data,
        finalize,
        hint,
        env,
    });
    let opaque = Box::into_raw(boxed);
    unsafe {
        qjs::JS_SetOpaque(obj, opaque as *mut c_void);
    }
    obj
}

pub fn get_external_pointer(val: JSValue) -> Option<*mut c_void> {
    if !unsafe { qjs::JS_IsObject(val) } {
        return None;
    }
    let class_id = EXTERNAL_CLASS_ID.get()?;
    let opaque = unsafe { qjs::JS_GetOpaque(val, *class_id) };
    if opaque.is_null() {
        return None;
    }
    let data = unsafe { &*(opaque as *const ExternalData) };
    Some(data.ptr)
}

pub fn is_external_object(val: JSValue) -> bool {
    if !unsafe { qjs::JS_IsObject(val) } {
        return false;
    }
    let Some(class_id) = EXTERNAL_CLASS_ID.get() else {
        return false;
    };
    !unsafe { qjs::JS_GetOpaque(val, *class_id) }.is_null()
}
