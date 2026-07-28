use std::collections::HashMap;
use std::os::raw::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use once_cell::sync::Lazy;
use parking_lot::Mutex;

static NEXT_ISOLATE_ID: AtomicUsize = AtomicUsize::new(1);
static NEXT_CONTEXT_ID: AtomicUsize = AtomicUsize::new(1);

static ISOLATE_IDS: Lazy<Mutex<HashMap<usize, usize>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static CONTEXT_IDS: Lazy<Mutex<HashMap<usize, usize>>> = Lazy::new(|| Mutex::new(HashMap::new()));

static CLEANUP_HOOKS: Lazy<Mutex<HashMap<usize, Vec<CleanupHook>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static CREATED_ISOLATES: AtomicUsize = AtomicUsize::new(0);
static DESTROYED_ISOLATES: AtomicUsize = AtomicUsize::new(0);
static CREATED_CONTEXTS: AtomicUsize = AtomicUsize::new(0);
static DESTROYED_CONTEXTS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy)]
struct CleanupHook {
    callback: unsafe extern "C" fn(*mut c_void),
    arg: usize,
}

pub fn isolate_key(isolate_ptr: usize) -> usize {
    let mut map = ISOLATE_IDS.lock();
    if let Some(&id) = map.get(&isolate_ptr) {
        return id;
    }
    let id = NEXT_ISOLATE_ID.fetch_add(1, Ordering::Relaxed);
    map.insert(isolate_ptr, id);
    CREATED_ISOLATES.fetch_add(1, Ordering::Relaxed);
    id
}

pub fn context_key(context_ptr: usize) -> usize {
    let mut map = CONTEXT_IDS.lock();
    if let Some(&id) = map.get(&context_ptr) {
        return id;
    }
    let id = NEXT_CONTEXT_ID.fetch_add(1, Ordering::Relaxed);
    map.insert(context_ptr, id);
    CREATED_CONTEXTS.fetch_add(1, Ordering::Relaxed);
    id
}

pub fn forget_isolate(isolate_ptr: usize) {
    if ISOLATE_IDS.lock().remove(&isolate_ptr).is_some() {
        DESTROYED_ISOLATES.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn forget_context(context_ptr: usize) {
    run_cleanup_hooks(context_ptr);
    if CONTEXT_IDS.lock().remove(&context_ptr).is_some() {
        DESTROYED_CONTEXTS.fetch_add(1, Ordering::Relaxed);
    }
    CLEANUP_HOOKS.lock().remove(&context_ptr);
}

pub fn add_cleanup_hook(scope_ptr: usize, cb: unsafe extern "C" fn(*mut c_void), arg: *mut c_void) {
    CLEANUP_HOOKS
        .lock()
        .entry(scope_ptr)
        .or_default()
        .push(CleanupHook {
            callback: cb,
            arg: arg as usize,
        });
}

pub fn remove_cleanup_hook(
    scope_ptr: usize,
    cb: unsafe extern "C" fn(*mut c_void),
    arg: *mut c_void,
) {
    let key = arg as usize;
    if let Some(hooks) = CLEANUP_HOOKS.lock().get_mut(&scope_ptr) {
        hooks.retain(|hook| !(hook.callback as usize == cb as usize && hook.arg == key));
    }
}

pub fn run_cleanup_hooks(scope_ptr: usize) {
    let hooks = CLEANUP_HOOKS.lock().remove(&scope_ptr).unwrap_or_default();
    for hook in hooks.into_iter().rev() {
        unsafe {
            (hook.callback)(hook.arg as *mut c_void);
        }
    }
}

pub fn lifecycle_counts() -> (usize, usize, usize, usize) {
    (
        CREATED_ISOLATES.load(Ordering::Relaxed),
        DESTROYED_ISOLATES.load(Ordering::Relaxed),
        CREATED_CONTEXTS.load(Ordering::Relaxed),
        DESTROYED_CONTEXTS.load(Ordering::Relaxed),
    )
}

#[cfg(test)]
pub fn reset_lifecycle_counters_for_tests() {
    CREATED_ISOLATES.store(0, Ordering::Relaxed);
    DESTROYED_ISOLATES.store(0, Ordering::Relaxed);
    CREATED_CONTEXTS.store(0, Ordering::Relaxed);
    DESTROYED_CONTEXTS.store(0, Ordering::Relaxed);
    ISOLATE_IDS.lock().clear();
    CONTEXT_IDS.lock().clear();
    CLEANUP_HOOKS.lock().clear();
}
