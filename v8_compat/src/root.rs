use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rquickjs::qjs::{self, JSValue};

static NEXT_ROOT_ID: AtomicU64 = AtomicU64::new(1);
static IMMORTAL_ROOTS: Lazy<Mutex<HashSet<u64>>> = Lazy::new(|| Mutex::new(HashSet::new()));

pub fn mark_immortal_root(id: u64) {
    if id != 0 {
        IMMORTAL_ROOTS.lock().insert(id);
    }
}

pub struct RootTable {
    entries: Mutex<HashMap<u64, JSValue>>,
}

impl RootTable {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, ctx: *mut qjs::JSContext, value: JSValue) -> u64 {
        let id = NEXT_ROOT_ID.fetch_add(1, Ordering::Relaxed);
        let dup = unsafe { qjs::JS_DupValue(ctx, value) };
        self.entries.lock().insert(id, dup);
        id
    }

    pub fn dup(&self, ctx: *mut qjs::JSContext, id: u64) -> Option<u64> {
        let entries = self.entries.lock();
        let value = entries.get(&id)?;
        let new_id = NEXT_ROOT_ID.fetch_add(1, Ordering::Relaxed);
        let dup = unsafe { qjs::JS_DupValue(ctx, *value) };
        drop(entries);
        self.entries.lock().insert(new_id, dup);
        Some(new_id)
    }

    pub fn get(&self, id: u64) -> Option<JSValue> {
        self.entries.lock().get(&id).copied()
    }

    pub fn drop_root(&self, ctx: *mut qjs::JSContext, id: u64) {
        if IMMORTAL_ROOTS.lock().contains(&id) {
            return;
        }
        if let Some(value) = self.entries.lock().remove(&id) {
            unsafe { qjs::JS_FreeValue(ctx, value) };
        }
    }

    pub fn find_id_by_ptr(&self, ptr: usize) -> Option<u64> {
        let entries = self.entries.lock();
        for (&id, &value) in entries.iter() {
            if unsafe { qjs::JS_IsObject(value) }
                && unsafe { qjs::JS_VALUE_GET_PTR(value) as usize } == ptr
            {
                return Some(id);
            }
        }
        None
    }

    pub fn clear(&self, ctx: *mut qjs::JSContext) {
        let mut entries = self.entries.lock();
        for (_, value) in entries.drain() {
            unsafe { qjs::JS_FreeValue(ctx, value) };
        }
        IMMORTAL_ROOTS.lock().clear();
    }
}

impl Default for RootTable {
    fn default() -> Self {
        Self::new()
    }
}
