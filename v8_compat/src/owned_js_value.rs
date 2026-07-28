//! RAII wrapper for JSValues with explicit ownership transfer into `RootTable`.

use std::mem::ManuallyDrop;

use rquickjs::qjs::{self, JSContext, JSValue};

/// A JSValue owned by Rust until consumed by `RootTable::insert_owned` or dropped.
pub struct OwnedJsValue {
    ctx: *mut JSContext,
    value: JSValue,
}

impl OwnedJsValue {
    /// Takes ownership of `value`; caller must not `JS_FreeValue` it afterward.
    pub unsafe fn new(ctx: *mut JSContext, value: JSValue) -> Self {
        Self { ctx, value }
    }

    pub fn as_value(&self) -> JSValue {
        self.value
    }

    /// Transfers ownership to the caller without running `Drop`.
    pub fn into_raw(self) -> JSValue {
        let this = ManuallyDrop::new(self);
        this.value
    }
}

impl Drop for OwnedJsValue {
    fn drop(&mut self) {
        unsafe { qjs::JS_FreeValue(self.ctx, self.value) };
    }
}
