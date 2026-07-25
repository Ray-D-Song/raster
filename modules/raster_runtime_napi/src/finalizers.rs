// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::os::raw::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use rquickjs::qjs::{self, JSContext, JSValue};

use crate::js_helpers::define_hidden_usize;
use crate::types::{napi_env, napi_finalize};

static NEXT_FINALIZER_ID: AtomicUsize = AtomicUsize::new(1);

pub struct FinalizerEntry {
    pub data: *mut c_void,
    pub finalize: napi_finalize,
    pub hint: *mut c_void,
}

pub struct FinalizerTable {
    by_id: HashMap<usize, FinalizerEntry>,
}

impl FinalizerTable {
    pub fn new() -> Self {
        Self {
            by_id: HashMap::new(),
        }
    }

    pub fn add(
        &mut self,
        ctx: *mut JSContext,
        obj: JSValue,
        data: *mut c_void,
        finalize: napi_finalize,
        hint: *mut c_void,
    ) -> bool {
        let id = NEXT_FINALIZER_ID.fetch_add(1, Ordering::Relaxed);
        if !unsafe { define_hidden_usize(ctx, obj, c"__napi_finalizer_id".as_ptr(), id) } {
            return false;
        }
        self.by_id.insert(
            id,
            FinalizerEntry {
                data,
                finalize,
                hint,
            },
        );
        true
    }

    pub fn run_all(&mut self, env: napi_env) {
        for (_, entry) in self.by_id.drain() {
            if let Some(f) = entry.finalize {
                unsafe { f(env, entry.data, entry.hint) };
            }
        }
    }
}
