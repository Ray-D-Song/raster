use once_cell::sync::Lazy;
use parking_lot::Mutex;

static ABI_TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn abi_test_lock() -> parking_lot::MutexGuard<'static, ()> {
    ABI_TEST_LOCK.lock()
}

#[test]
fn abi_profile_constants() {
    assert_eq!(crate::NODE_MODULE_VERSION, 137);
    assert_eq!(crate::NODE_VERSION, "24.3.0");
    assert_eq!(crate::ABI_PROFILE_LABEL, "node24-abi137");
}

#[test]
fn runtime_context_lifecycle_counts_match() {
    let _lock = abi_test_lock();
    use rquickjs::{Context, Runtime};

    crate::runtime_state::reset_lifecycle_counters_for_tests();
    let (ci0, di0, cc0, dc0) = crate::runtime_state::lifecycle_counts();
    let runtime = Runtime::new().expect("runtime");
    let context = Context::full(&runtime).expect("context");
    let ctx_ptr = context.as_raw().as_ptr();
    crate::bind_bridge(ctx_ptr);
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ctx_ptr) };
    let _isolate = crate::ensure_isolate_for_runtime(qrt);
    let _context = crate::ensure_context_for_ctx(ctx_ptr);
    unsafe { crate::shutdown_context(ctx_ptr) };
    unsafe { crate::shutdown_runtime(qrt) };
    let (ci1, di1, cc1, dc1) = crate::runtime_state::lifecycle_counts();
    assert_eq!(ci1 - ci0, di1 - di0);
    assert_eq!(cc1 - cc0, dc1 - dc0);
    assert!(ci1 > ci0);
    assert!(cc1 > cc0);
}

#[test]
fn per_context_tables_do_not_leak_across_contexts() {
    let _lock = abi_test_lock();
    use rquickjs::{Context, Runtime};

    let runtime = Runtime::new().expect("runtime");
    let ctx_a = Context::full(&runtime).expect("ctx_a");
    let ctx_b = Context::full(&runtime).expect("ctx_b");
    let ptr_a = ctx_a.as_raw().as_ptr();
    let ptr_b = ctx_b.as_raw().as_ptr();
    crate::bind_bridge(ptr_a);

    let key_a = unsafe {
        let obj = rquickjs::qjs::JS_NewObject(ptr_a);
        let key = rquickjs::qjs::JS_VALUE_GET_PTR(obj) as usize;
        rquickjs::qjs::JS_FreeValue(ptr_a, obj);
        key
    };
    crate::context_tables::with_context_tables(ptr_a, |tables| {
        tables.internal_fields.insert(key_a, vec![42]);
    });

    let count_b =
        crate::context_tables::with_context_tables(ptr_b, |tables| tables.internal_fields.len());
    assert_eq!(count_b, 0);

    unsafe { crate::shutdown_context(ptr_a) };
    unsafe { crate::shutdown_context(ptr_b) };
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ptr_a) };
    unsafe { crate::shutdown_runtime(qrt) };
}

#[test]
fn cleanup_hook_runs_for_context_scope() {
    let _lock = abi_test_lock();
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CALLED: AtomicUsize = AtomicUsize::new(0);
    unsafe extern "C" fn hook(_arg: *mut std::ffi::c_void) {
        CALLED.fetch_add(1, Ordering::Relaxed);
    }

    let context_ptr = 0xCAFE_BABEusize;
    crate::runtime_state::context_key(context_ptr);
    crate::runtime_state::add_cleanup_hook(context_ptr, hook, std::ptr::null_mut());
    crate::runtime_state::forget_context(context_ptr);
    assert_eq!(CALLED.load(Ordering::Relaxed), 1);
}

#[test]
fn pending_weak_queue_is_per_context() {
    let _lock = abi_test_lock();
    use rquickjs::{Context, Runtime};

    let runtime = Runtime::new().expect("runtime");
    let ctx_a = Context::full(&runtime).expect("ctx_a");
    let ctx_b = Context::full(&runtime).expect("ctx_b");
    let ptr_a = ctx_a.as_raw().as_ptr();
    let ptr_b = ctx_b.as_raw().as_ptr();

    crate::context_tables::with_context_tables(ptr_a, |tables| {
        tables
            .pending_weak
            .push(crate::context_tables::PendingWeak {
                callback: 1,
                parameter: 2,
                object_key: 3,
                pass: crate::context_tables::WeakPass::First,
            });
    });

    let pending_b = crate::context_tables::take_pending_weak_callbacks(ptr_b);
    assert!(pending_b.is_empty());
    let pending_a = crate::context_tables::take_pending_weak_callbacks(ptr_a);
    assert_eq!(pending_a.len(), 1);

    unsafe { crate::shutdown_context(ptr_a) };
    unsafe { crate::shutdown_context(ptr_b) };
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ptr_a) };
    unsafe { crate::shutdown_runtime(qrt) };
}

#[test]
fn cleanup_hook_add_and_remove() {
    let _lock = abi_test_lock();
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CALLED: AtomicUsize = AtomicUsize::new(0);
    unsafe extern "C" fn hook(_arg: *mut std::ffi::c_void) {
        CALLED.fetch_add(1, Ordering::Relaxed);
    }

    let isolate_ptr = 0xDEAD_BEEFusize;
    crate::runtime_state::add_cleanup_hook(isolate_ptr, hook, std::ptr::null_mut());
    crate::runtime_state::remove_cleanup_hook(isolate_ptr, hook, std::ptr::null_mut());
    crate::runtime_state::run_cleanup_hooks(isolate_ptr);
    assert_eq!(CALLED.load(Ordering::Relaxed), 0);

    crate::runtime_state::add_cleanup_hook(isolate_ptr, hook, std::ptr::null_mut());
    crate::runtime_state::run_cleanup_hooks(isolate_ptr);
    assert_eq!(CALLED.load(Ordering::Relaxed), 1);
}

#[test]
fn bridge_roots_are_per_context() {
    let _lock = abi_test_lock();
    use rquickjs::{Context, Runtime};

    let runtime = Runtime::new().expect("runtime");
    let ctx_a = Context::full(&runtime).expect("ctx_a");
    let ctx_b = Context::full(&runtime).expect("ctx_b");
    let ptr_a = ctx_a.as_raw().as_ptr();
    let ptr_b = ctx_b.as_raw().as_ptr();
    crate::bind_bridge(ptr_a);
    crate::bind_bridge(ptr_b);

    crate::bridge::set_active_bridge_context(ptr_a);
    let root_a = crate::bridge::with_bridge_roots(|ctx, roots| {
        let obj = unsafe { rquickjs::qjs::JS_NewObject(ctx) };
        let root = roots.insert_borrowed(ctx, obj);
        unsafe { rquickjs::qjs::JS_FreeValue(ctx, obj) };
        root
    });

    let visible_in_b =
        crate::bridge::with_state_ref_for_ctx(ptr_b, |state| state.roots.get(root_a).is_some());
    assert!(!visible_in_b);

    unsafe { crate::shutdown_context(ptr_a) };
    unsafe { crate::shutdown_context(ptr_b) };
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ptr_a) };
    unsafe { crate::shutdown_runtime(qrt) };
}

#[test]
fn context_tables_survive_sibling_shutdown() {
    let _lock = abi_test_lock();
    use rquickjs::{Context, Runtime};

    let runtime = Runtime::new().expect("runtime");
    let ctx_a = Context::full(&runtime).expect("ctx_a");
    let ctx_b = Context::full(&runtime).expect("ctx_b");
    let ptr_a = ctx_a.as_raw().as_ptr();
    let ptr_b = ctx_b.as_raw().as_ptr();
    crate::bind_bridge(ptr_a);
    crate::bind_bridge(ptr_b);

    let key_b = unsafe {
        let obj = rquickjs::qjs::JS_NewObject(ptr_b);
        let key = rquickjs::qjs::JS_VALUE_GET_PTR(obj) as usize;
        rquickjs::qjs::JS_FreeValue(ptr_b, obj);
        key
    };
    crate::context_tables::with_context_tables(ptr_b, |tables| {
        tables.internal_fields.insert(key_b, vec![99]);
    });

    unsafe { crate::bridge::prepare_shutdown(ptr_a) };

    let still_there = crate::context_tables::with_context_tables(ptr_b, |tables| {
        tables.internal_fields.get(&key_b).cloned()
    });
    assert_eq!(still_there, Some(vec![99]));

    unsafe { crate::shutdown_context(ptr_a) };
    unsafe { crate::shutdown_context(ptr_b) };
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ptr_a) };
    unsafe { crate::shutdown_runtime(qrt) };
}

#[test]
fn dual_context_shutdown_preserves_sibling_until_runtime_free() {
    let _lock = abi_test_lock();
    use rquickjs::{Context, Runtime};

    let runtime = Runtime::new().expect("runtime");
    let ctx_a = Context::full(&runtime).expect("ctx_a");
    let ctx_b = Context::full(&runtime).expect("ctx_b");
    let ptr_a = ctx_a.as_raw().as_ptr();
    let ptr_b = ctx_b.as_raw().as_ptr();
    crate::bind_bridge(ptr_a);
    crate::bind_bridge(ptr_b);
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ptr_a) };

    let obj_a = crate::js_ops::new_v8_object(ptr_a);
    unsafe { rquickjs::qjs::JS_FreeValue(ptr_a, obj_a) };
    assert!(crate::js_ops::v8_object_class_for_runtime(qrt).is_some());

    unsafe { crate::shutdown_context(ptr_a) };

    crate::bridge::set_active_bridge_context(ptr_b);
    let root_b = crate::bridge::with_bridge_roots(|ctx, roots| {
        let obj = unsafe { rquickjs::qjs::JS_NewObject(ctx) };
        let root = roots.insert_borrowed(ctx, obj);
        unsafe { rquickjs::qjs::JS_FreeValue(ctx, obj) };
        root
    });
    assert!(crate::bridge::with_state_ref_for_ctx(ptr_b, |state| state
        .roots
        .get(root_b)
        .is_some()));

    let obj_b = crate::js_ops::new_v8_object(ptr_b);
    assert!(!unsafe { rquickjs::qjs::JS_IsException(obj_b) });
    unsafe { rquickjs::qjs::JS_FreeValue(ptr_b, obj_b) };

    unsafe { crate::shutdown_context(ptr_b) };
    unsafe { crate::shutdown_runtime(qrt) };
    assert!(crate::js_ops::v8_object_class_for_runtime(qrt).is_none());
}

#[test]
fn v8_object_class_cleared_after_dual_context_shutdown() {
    let _lock = abi_test_lock();
    use rquickjs::{Context, Runtime};

    let runtime = Runtime::new().expect("runtime");
    let ctx_a = Context::full(&runtime).expect("ctx_a");
    let ctx_b = Context::full(&runtime).expect("ctx_b");
    let ptr_a = ctx_a.as_raw().as_ptr();
    let ptr_b = ctx_b.as_raw().as_ptr();
    crate::bind_bridge(ptr_a);
    crate::bind_bridge(ptr_b);

    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ptr_a) };
    let obj_a = crate::js_ops::new_v8_object(ptr_a);
    let obj_b = crate::js_ops::new_v8_object(ptr_b);
    assert!(crate::js_ops::v8_object_class_for_runtime(qrt).is_some());
    unsafe {
        rquickjs::qjs::JS_FreeValue(ptr_a, obj_a);
        rquickjs::qjs::JS_FreeValue(ptr_b, obj_b);
    }

    unsafe { crate::shutdown_context(ptr_a) };
    unsafe { crate::shutdown_context(ptr_b) };
    unsafe { crate::shutdown_runtime(qrt) };
    assert!(crate::js_ops::v8_object_class_for_runtime(qrt).is_none());
}

#[test]
fn buffer_copy_zero_length_allows_null_data() {
    let _lock = abi_test_lock();
    use rquickjs::{Context, Runtime};

    let runtime = Runtime::new().expect("runtime");
    let context = Context::full(&runtime).expect("context");
    let ctx_ptr = context.as_raw().as_ptr();
    crate::bind_bridge(ctx_ptr);
    let _state = crate::ensure_context_for_ctx(ctx_ptr);
    crate::bridge::set_active_bridge_context(ctx_ptr);

    let global = unsafe { rquickjs::qjs::JS_GetGlobalObject(ctx_ptr) };
    let has_buffer = unsafe {
        !rquickjs::qjs::JS_IsUndefined(rquickjs::qjs::JS_GetPropertyStr(
            ctx_ptr,
            global,
            c"Buffer".as_ptr(),
        ))
    };
    unsafe { rquickjs::qjs::JS_FreeValue(ctx_ptr, global) };
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ctx_ptr) };
    if !has_buffer {
        unsafe { crate::shutdown_context(ctx_ptr) };
        unsafe { crate::shutdown_runtime(qrt) };
        return;
    }

    let mut root: u64 = 0;
    let status = unsafe {
        crate::value_ops::buffer_new_copy(std::ptr::null_mut(), std::ptr::null(), 0, &mut root)
    };
    assert!(matches!(status, crate::bridge::RasterV8Status::Ok));
    assert_ne!(root, 0);

    unsafe { crate::shutdown_context(ctx_ptr) };
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ctx_ptr) };
    unsafe { crate::shutdown_runtime(qrt) };
}

#[test]
fn current_thread_locals_cleared_after_teardown() {
    let _lock = abi_test_lock();
    use rquickjs::{Context, Runtime};

    extern "C" {
        fn raster_v8_current_context() -> *mut crate::bridge::RasterV8ContextState;
        fn raster_v8_current_isolate() -> *mut crate::bridge::RasterV8IsolateState;
        fn raster_v8_set_current_context(ctx: *mut crate::bridge::RasterV8ContextState);
        fn raster_v8_set_current_isolate(isolate: *mut crate::bridge::RasterV8IsolateState);
    }

    let runtime = Runtime::new().expect("runtime");
    let context = Context::full(&runtime).expect("context");
    let ctx_ptr = context.as_raw().as_ptr();
    crate::bind_bridge(ctx_ptr);
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ctx_ptr) };
    let isolate = crate::ensure_isolate_for_runtime(qrt);
    let context_state = crate::ensure_context_for_ctx(ctx_ptr);

    unsafe {
        raster_v8_set_current_context(context_state as *mut crate::bridge::RasterV8ContextState);
        raster_v8_set_current_isolate(isolate as *mut crate::bridge::RasterV8IsolateState);
    }
    assert!(!unsafe { raster_v8_current_context().is_null() });
    assert!(!unsafe { raster_v8_current_isolate().is_null() });

    unsafe { crate::shutdown_context(ctx_ptr) };
    assert!(unsafe { raster_v8_current_context().is_null() });

    unsafe { crate::shutdown_runtime(qrt) };
    assert!(unsafe { raster_v8_current_isolate().is_null() });
}

#[test]
fn isolate_cleanup_hook_runs_only_at_runtime_shutdown() {
    let _lock = abi_test_lock();
    use rquickjs::{Context, Runtime};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CALLED: AtomicUsize = AtomicUsize::new(0);
    unsafe extern "C" fn hook(_arg: *mut std::ffi::c_void) {
        CALLED.fetch_add(1, Ordering::Relaxed);
    }

    let runtime = Runtime::new().expect("runtime");
    let ctx_a = Context::full(&runtime).expect("ctx_a");
    let ctx_b = Context::full(&runtime).expect("ctx_b");
    let ptr_a = ctx_a.as_raw().as_ptr();
    let ptr_b = ctx_b.as_raw().as_ptr();
    crate::bind_bridge(ptr_a);
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ptr_a) };
    let isolate = crate::ensure_isolate_for_runtime(qrt);
    let _ctx_a = crate::ensure_context_for_ctx(ptr_a);
    let _ctx_b = crate::ensure_context_for_ctx(ptr_b);

    crate::runtime_state::add_cleanup_hook(isolate as usize, hook, std::ptr::null_mut());
    CALLED.store(0, Ordering::Relaxed);

    unsafe { crate::shutdown_context(ptr_a) };
    assert_eq!(CALLED.load(Ordering::Relaxed), 0);

    unsafe { crate::shutdown_context(ptr_b) };
    assert_eq!(CALLED.load(Ordering::Relaxed), 0);

    unsafe { crate::shutdown_runtime(qrt) };
    assert_eq!(CALLED.load(Ordering::Relaxed), 1);
}
