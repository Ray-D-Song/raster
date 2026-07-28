//! Per-`JSContext` side tables for V8 object internal fields and weak callbacks.

use std::collections::HashMap;

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rquickjs::qjs::{self, JSContext};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeakPhase {
    Registered,
    PendingFirstPass,
    PendingSecondPass,
    Disposed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeakPass {
    First,
    Second,
}

#[derive(Clone, Copy)]
pub struct WeakSlot {
    pub callback: usize,
    pub parameter: usize,
    pub phase: WeakPhase,
}

#[derive(Default)]
pub struct ContextJsTables {
    pub internal_fields: HashMap<usize, Vec<usize>>,
    pub internal_fields_by_root: HashMap<u64, Vec<usize>>,
    pub object_field_counts: HashMap<usize, usize>,
    pub weak_callbacks: HashMap<usize, WeakSlot>,
    pub pending_weak: Vec<PendingWeak>,
}

static TABLES: Lazy<Mutex<HashMap<usize, ContextJsTables>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn ctx_key(ctx: *mut JSContext) -> usize {
    ctx as usize
}

pub fn with_context_tables<F, R>(ctx: *mut JSContext, f: F) -> R
where
    F: FnOnce(&mut ContextJsTables) -> R,
{
    let key = ctx_key(ctx);
    let mut guard = TABLES.lock();
    f(guard.entry(key).or_default())
}

pub fn remove_context_tables(ctx: *mut JSContext) {
    TABLES.lock().remove(&ctx_key(ctx));
}

pub fn clear_context_tables(ctx: *mut JSContext) {
    if let Some(entry) = TABLES.lock().get_mut(&ctx_key(ctx)) {
        *entry = ContextJsTables::default();
    }
}

pub fn remove_context_tables_for_runtime(rt: *mut qjs::JSRuntime) {
    let rt_key = rt as usize;
    let stale: Vec<usize> = TABLES
        .lock()
        .keys()
        .copied()
        .filter(|ctx_key| {
            let ctx = *ctx_key as *mut JSContext;
            unsafe { qjs::JS_GetRuntime(ctx) as usize == rt_key }
        })
        .collect();
    let mut guard = TABLES.lock();
    for key in stale {
        guard.remove(&key);
    }
}

pub fn remove_object_records(rt: *mut qjs::JSRuntime, object_key: usize) {
    let rt_key = rt as usize;
    let mut guard = TABLES.lock();
    for (ctx_key, tables) in guard.iter_mut() {
        let ctx = *ctx_key as *mut JSContext;
        if unsafe { qjs::JS_GetRuntime(ctx) as usize != rt_key } {
            continue;
        }
        tables.internal_fields.remove(&object_key);
        tables.object_field_counts.remove(&object_key);
        if let Some(slot) = tables.weak_callbacks.remove(&object_key) {
            if slot.callback != 0 && slot.phase != WeakPhase::Disposed {
                tables.pending_weak.push(PendingWeak {
                    callback: slot.callback,
                    parameter: slot.parameter,
                    object_key,
                    pass: WeakPass::First,
                });
            }
        }
    }
}

pub(crate) struct PendingWeak {
    pub callback: usize,
    pub parameter: usize,
    pub object_key: usize,
    pub pass: WeakPass,
}

pub fn take_pending_weak_callbacks(ctx: *mut JSContext) -> Vec<PendingWeak> {
    with_context_tables(ctx, |tables| std::mem::take(&mut tables.pending_weak))
}

pub fn requeue_pending_weak_callbacks(ctx: *mut JSContext, items: Vec<PendingWeak>) {
    if items.is_empty() {
        return;
    }
    with_context_tables(ctx, |tables| tables.pending_weak.extend(items));
}

pub fn cancel_pending_weak_for_object(ctx: *mut JSContext, object_key: usize) {
    if object_key == 0 {
        return;
    }
    with_context_tables(ctx, |tables| {
        tables
            .pending_weak
            .retain(|item| item.object_key != object_key);
    });
}

pub fn unregister_weak_callback(ctx: *mut JSContext, object_key: usize) {
    if object_key == 0 {
        return;
    }
    with_context_tables(ctx, |tables| {
        if let Some(slot) = tables.weak_callbacks.get_mut(&object_key) {
            slot.phase = WeakPhase::Disposed;
        }
        tables.weak_callbacks.remove(&object_key);
    });
    cancel_pending_weak_for_object(ctx, object_key);
}
