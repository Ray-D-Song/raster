// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use rquickjs::qjs::{self, JSContext, JSValue};

use crate::value::NapiValue;

pub struct HandleScope {
    pub values: Vec<JSValue>,
    /// Heap-allocated `NapiValue` handles; freed when the scope closes.
    pub handles: Vec<*mut NapiValue>,
}

pub struct EscapableHandleScope {
    pub values: Vec<JSValue>,
    pub handles: Vec<*mut NapiValue>,
    pub escaped: bool,
}

pub struct ScopeStack {
    scopes: Vec<HandleScope>,
    escapable: Vec<EscapableHandleScope>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            scopes: Vec::new(),
            escapable: Vec::new(),
        }
    }

    pub fn open(&mut self) {
        self.scopes.push(HandleScope {
            values: Vec::new(),
            handles: Vec::new(),
        });
    }

    pub fn close(&mut self, ctx: *mut JSContext) {
        if let Some(scope) = self.scopes.pop() {
            free_scope(ctx, scope);
        }
    }

    pub fn open_escapable(&mut self) {
        self.escapable.push(EscapableHandleScope {
            values: Vec::new(),
            handles: Vec::new(),
            escaped: false,
        });
    }

    pub fn close_escapable(&mut self, ctx: *mut JSContext) {
        if let Some(scope) = self.escapable.pop() {
            for value in scope.values {
                unsafe {
                    qjs::JS_FreeValue(ctx, value);
                }
            }
            for ptr in scope.handles {
                unsafe {
                    let _ = Box::from_raw(ptr);
                }
            }
        }
    }

    /// Root `value` in the parent handle scope (outside the innermost escapable scope).
    pub fn push_to_parent_handle_scope(&mut self, value: JSValue) -> usize {
        let parent = self
            .scopes
            .last_mut()
            .expect("parent handle scope required for escape");
        let idx = parent.values.len();
        parent.values.push(value);
        idx
    }

    pub fn push_handle_to_parent(&mut self, ptr: *mut NapiValue) {
        self.scopes
            .last_mut()
            .expect("parent handle scope required for escape")
            .handles
            .push(ptr);
    }

    pub fn current_mut(&mut self) -> Option<&mut HandleScope> {
        self.scopes.last_mut()
    }

    pub fn current_escapable_mut(&mut self) -> Option<&mut EscapableHandleScope> {
        self.escapable.last_mut()
    }

    pub fn depth(&self) -> usize {
        self.scopes.len()
    }

    pub fn escapable_depth(&self) -> usize {
        self.escapable.len()
    }

    pub fn resolve_value(&self, value_index: usize, in_parent_scope: bool) -> Option<JSValue> {
        if in_parent_scope {
            return self.scopes.last()?.values.get(value_index).copied();
        }
        if let Some(esc) = self.escapable.last() {
            if let Some(v) = esc.values.get(value_index) {
                return Some(*v);
            }
        }
        self.scopes.last()?.values.get(value_index).copied()
    }
}

fn free_scope(ctx: *mut JSContext, scope: HandleScope) {
    for value in scope.values {
        unsafe {
            qjs::JS_FreeValue(ctx, value);
        }
    }
    for ptr in scope.handles {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}
