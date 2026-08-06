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
    unsafe { crate::shutdown_context(ctx_ptr).unwrap() };
    unsafe { crate::shutdown_runtime(qrt).unwrap() };
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

    unsafe { crate::shutdown_context(ptr_a).unwrap() };
    unsafe { crate::shutdown_context(ptr_b).unwrap() };
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ptr_a) };
    unsafe { crate::shutdown_runtime(qrt).unwrap() };
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

    unsafe { crate::shutdown_context(ptr_a).unwrap() };
    unsafe { crate::shutdown_context(ptr_b).unwrap() };
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ptr_a) };
    unsafe { crate::shutdown_runtime(qrt).unwrap() };
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

    unsafe { crate::shutdown_context(ptr_a).unwrap() };
    unsafe { crate::shutdown_context(ptr_b).unwrap() };
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ptr_a) };
    unsafe { crate::shutdown_runtime(qrt).unwrap() };
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

    unsafe { crate::bridge::prepare_shutdown(ptr_a).unwrap() };

    let still_there = crate::context_tables::with_context_tables(ptr_b, |tables| {
        tables.internal_fields.get(&key_b).cloned()
    });
    assert_eq!(still_there, Some(vec![99]));

    unsafe { crate::shutdown_context(ptr_a).unwrap() };
    unsafe { crate::shutdown_context(ptr_b).unwrap() };
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ptr_a) };
    unsafe { crate::shutdown_runtime(qrt).unwrap() };
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

    unsafe { crate::shutdown_context(ptr_a).unwrap() };

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

    unsafe { crate::shutdown_context(ptr_b).unwrap() };
    unsafe { crate::shutdown_runtime(qrt).unwrap() };
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

    unsafe { crate::shutdown_context(ptr_a).unwrap() };
    unsafe { crate::shutdown_context(ptr_b).unwrap() };
    unsafe { crate::shutdown_runtime(qrt).unwrap() };
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
        unsafe { crate::shutdown_context(ctx_ptr).unwrap() };
        unsafe { crate::shutdown_runtime(qrt).unwrap() };
        return;
    }

    let mut root: u64 = 0;
    let status = unsafe {
        crate::value_ops::buffer_new_copy(std::ptr::null_mut(), std::ptr::null(), 0, &mut root)
    };
    assert!(matches!(status, crate::bridge::RasterV8Status::Ok));
    assert_ne!(root, 0);

    unsafe { crate::shutdown_context(ctx_ptr).unwrap() };
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ctx_ptr) };
    unsafe { crate::shutdown_runtime(qrt).unwrap() };
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

    unsafe { crate::shutdown_context(ctx_ptr).unwrap() };
    assert!(unsafe { raster_v8_current_context().is_null() });

    unsafe { crate::shutdown_runtime(qrt).unwrap() };
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

    unsafe { crate::shutdown_context(ptr_a).unwrap() };
    assert_eq!(CALLED.load(Ordering::Relaxed), 0);

    unsafe { crate::shutdown_context(ptr_b).unwrap() };
    assert_eq!(CALLED.load(Ordering::Relaxed), 0);

    unsafe { crate::shutdown_runtime(qrt).unwrap() };
    assert_eq!(CALLED.load(Ordering::Relaxed), 1);
}

#[test]
fn shutdown_context_clears_bridge_and_context_registry() {
    let _lock = abi_test_lock();
    use rquickjs::{Context, Runtime};

    let runtime = Runtime::new().expect("runtime");
    let context = Context::full(&runtime).expect("context");
    let ctx_ptr = context.as_raw().as_ptr();
    crate::bind_bridge(ctx_ptr);
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ctx_ptr) };
    let _isolate = crate::ensure_isolate_for_runtime(qrt);
    let _ctx_state = crate::ensure_context_for_ctx(ctx_ptr);

    unsafe { crate::shutdown_context(ctx_ptr).unwrap() };
    assert!(!crate::bridge::has_bridge_for_ctx(ctx_ptr));
    assert_eq!(crate::contexts_for_runtime(qrt), 0);
    assert_eq!(crate::bridge::residual_root_count_for_runtime(qrt), 0);

    unsafe { crate::shutdown_runtime(qrt).unwrap() };
}

#[test]
fn repeated_shutdown_is_idempotent() {
    let _lock = abi_test_lock();
    use rquickjs::{Context, Runtime};

    let runtime = Runtime::new().expect("runtime");
    let context = Context::full(&runtime).expect("context");
    let ctx_ptr = context.as_raw().as_ptr();
    crate::bind_bridge(ctx_ptr);
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ctx_ptr) };
    let _isolate = crate::ensure_isolate_for_runtime(qrt);
    let _ctx_state = crate::ensure_context_for_ctx(ctx_ptr);

    unsafe { crate::shutdown_context(ctx_ptr).unwrap() };
    unsafe { crate::shutdown_context(ctx_ptr).unwrap() };
    unsafe { crate::shutdown_runtime(qrt).unwrap() };
    unsafe { crate::shutdown_runtime(qrt).unwrap() };
}

#[test]
fn cleanup_hooks_run_lifo_once_per_scope() {
    let _lock = abi_test_lock();
    use std::sync::atomic::{AtomicUsize, Ordering};

    static ORDER: AtomicUsize = AtomicUsize::new(0);
    static LOG: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());

    unsafe extern "C" fn hook_a(_arg: *mut std::ffi::c_void) {
        LOG.lock().unwrap().push(1);
        ORDER.fetch_add(1, Ordering::SeqCst);
    }
    unsafe extern "C" fn hook_b(_arg: *mut std::ffi::c_void) {
        LOG.lock().unwrap().push(2);
        ORDER.fetch_add(1, Ordering::SeqCst);
    }

    let context_ptr = 0xDEAD_BEEFusize;
    crate::runtime_state::context_key(context_ptr);
    crate::runtime_state::add_cleanup_hook(context_ptr, hook_a, std::ptr::null_mut());
    crate::runtime_state::add_cleanup_hook(context_ptr, hook_b, std::ptr::null_mut());
    crate::runtime_state::forget_context(context_ptr);

    let log = LOG.lock().unwrap().clone();
    assert_eq!(log, vec![2, 1]);
    assert_eq!(ORDER.load(Ordering::SeqCst), 2);
}

#[test]
fn objectwrap_fixture_teardown_clears_all_counts() {
    let _lock = abi_test_lock();
    use rquickjs::{Context, Runtime};

    extern "C" {
        fn raster_v8_test_objectwrap_fixture_setup(
            ctx: *mut crate::bridge::RasterV8ContextState,
        ) -> *mut std::ffi::c_void;
        fn raster_v8_test_objectwrap_fixture_release_bridge_roots(
            ctx: *mut crate::bridge::RasterV8ContextState,
            counters: *mut std::ffi::c_void,
        );
        fn raster_v8_test_objectwrap_fixture_cleanup_hook(arg: *mut std::ffi::c_void);
        fn raster_v8_test_objectwrap_fixture_read_counts(
            counters: *const std::ffi::c_void,
            cleanup_out: *mut i32,
            weak_out: *mut i32,
            destructor_out: *mut i32,
        );
        fn raster_v8_test_objectwrap_fixture_destroy(counters: *mut std::ffi::c_void);
    }

    let runtime = Runtime::new().expect("runtime");
    let context = Context::full(&runtime).expect("context");
    let ctx_ptr = context.as_raw().as_ptr();
    crate::bind_bridge(ctx_ptr);
    let qrt = unsafe { rquickjs::qjs::JS_GetRuntime(ctx_ptr) };
    let _isolate = crate::ensure_isolate_for_runtime(qrt);
    let context_state = crate::ensure_context_for_ctx(ctx_ptr);
    crate::bridge::set_active_bridge_context(ctx_ptr);

    extern "C" {
        fn raster_v8_set_current_context(ctx: *mut crate::bridge::RasterV8ContextState);
        fn raster_v8_set_current_isolate(isolate: *mut crate::bridge::RasterV8IsolateState);
    }
    unsafe {
        raster_v8_set_current_context(context_state as *mut crate::bridge::RasterV8ContextState);
        raster_v8_set_current_isolate(_isolate as *mut crate::bridge::RasterV8IsolateState);
    }

    let counters = unsafe {
        raster_v8_test_objectwrap_fixture_setup(
            context_state as *mut crate::bridge::RasterV8ContextState,
        )
    };
    assert!(!counters.is_null());
    unsafe {
        crate::runtime_state::add_cleanup_hook(
            context_state as usize,
            raster_v8_test_objectwrap_fixture_cleanup_hook,
            counters as *mut std::ffi::c_void,
        );
        raster_v8_test_objectwrap_fixture_release_bridge_roots(
            context_state as *mut crate::bridge::RasterV8ContextState,
            counters,
        );
    }

    for _ in 0..8 {
        unsafe {
            rquickjs::qjs::JS_RunGC(qrt);
        }
        crate::js_ops::dispatch_pending_weak_callbacks_for_ctx(ctx_ptr);
    }

    unsafe { crate::shutdown_context(ctx_ptr).unwrap() };

    let mut cleanup = 0;
    let mut weak = 0;
    let mut dtor = 0;
    unsafe {
        raster_v8_test_objectwrap_fixture_read_counts(counters, &mut cleanup, &mut weak, &mut dtor);
    }
    assert_eq!(cleanup, 1, "cleanup hook should run once");
    assert_eq!(weak, 1, "weak ObjectWrap callback should run once");
    assert_eq!(dtor, 1, "native destructor should run once");
    assert!(
        crate::bridge::teardown_counts_for_ctx(ctx_ptr).is_zero(),
        "teardown counts should be zero"
    );

    unsafe {
        raster_v8_test_objectwrap_fixture_destroy(counters);
    }
    // Leak Context/Runtime: this fixture validates V8 bridge teardown only.
    std::mem::forget(context);
    std::mem::forget(runtime);
}

struct WiredTestContext {
    runtime: rquickjs::Runtime,
    context: rquickjs::Context,
    ctx_ptr: *mut rquickjs::qjs::JSContext,
    rt_ptr: *mut rquickjs::qjs::JSRuntime,
    context_state: *mut crate::bridge::RasterV8ContextState,
    pending_missing_key: bool,
}

impl WiredTestContext {
    fn new() -> Self {
        let runtime = rquickjs::Runtime::new().expect("runtime");
        let context = rquickjs::Context::full(&runtime).expect("context");
        let ctx_ptr = context.as_raw().as_ptr();
        crate::bind_bridge(ctx_ptr);
        let rt_ptr = unsafe { rquickjs::qjs::JS_GetRuntime(ctx_ptr) };
        let _isolate = crate::ensure_isolate_for_runtime(rt_ptr);
        let context_state =
            crate::ensure_context_for_ctx(ctx_ptr) as *mut crate::bridge::RasterV8ContextState;
        crate::bridge::set_active_bridge_context(ctx_ptr);
        extern "C" {
            fn raster_v8_set_current_context(ctx: *mut crate::bridge::RasterV8ContextState);
            fn raster_v8_set_current_isolate(isolate: *mut crate::bridge::RasterV8IsolateState);
        }
        unsafe {
            raster_v8_set_current_context(context_state);
            raster_v8_set_current_isolate(_isolate as *mut crate::bridge::RasterV8IsolateState);
        }
        Self {
            runtime,
            context,
            ctx_ptr,
            rt_ptr,
            context_state,
            pending_missing_key: false,
        }
    }

    fn register_template_property_with_missing_key(&mut self) {
        self.pending_missing_key = true;
    }

    fn register_function_template(&mut self) -> u32 {
        extern "C" {
            fn raster_v8_test_register_function_template() -> u32;
            fn raster_v8_test_register_template_property_with_missing_key(function_id: u32);
        }
        let function_id = unsafe { raster_v8_test_register_function_template() };
        if self.pending_missing_key {
            unsafe { raster_v8_test_register_template_property_with_missing_key(function_id) };
            self.pending_missing_key = false;
        }
        function_id
    }

    fn get_function(&mut self, function_id: u32) -> u64 {
        extern "C" {
            fn raster_v8_test_function_template_get_function(
                ctx_state: *mut crate::bridge::RasterV8ContextState,
                function_id: u32,
                out_root: *mut u64,
            ) -> crate::bridge::RasterV8Status;
        }
        let mut root = 0u64;
        let status = unsafe {
            raster_v8_test_function_template_get_function(
                self.context_state as *mut crate::bridge::RasterV8ContextState,
                function_id,
                &mut root,
            )
        };
        assert!(matches!(status, crate::bridge::RasterV8Status::Ok));
        root
    }

    fn function_root(&self, function_id: u32) -> Option<u64> {
        crate::bridge::with_state_ref_for_ctx(self.ctx_ptr, |state| {
            crate::bridge::function_root_for_id(state, function_id)
        })
    }

    fn template_prototype(&self, function_id: u32) -> rquickjs::qjs::JSValue {
        extern "C" {
            fn raster_v8_function_template_id(function_id: u32) -> u32;
            fn raster_v8_function_template_prototype_root(template_id: u32) -> u64;
        }
        let template_id = unsafe { raster_v8_function_template_id(function_id) };
        assert_ne!(
            template_id, 0,
            "function_id must map to a function template"
        );
        let proto_root = unsafe { raster_v8_function_template_prototype_root(template_id) };
        assert_ne!(proto_root, 0);
        crate::bridge::with_state_ref_for_ctx(self.ctx_ptr, |state| {
            state.roots.get(proto_root).expect("prototype root")
        })
    }

    fn bridge_strong_root_count(&self) -> usize {
        crate::bridge::teardown_counts_for_ctx(self.ctx_ptr).strong_roots
    }

    /// Runs bridge `shutdown_runtime`, then intentionally leaks the rquickjs
    /// `Context`/`Runtime` handles.
    ///
    /// These unit tests validate V8 bridge teardown only. Empty `gc_obj_list`
    /// at `JS_FreeRuntime` is covered by `make compat-v8`.
    fn shutdown_bridge_and_leak_runtime(self) {
        unsafe {
            crate::shutdown_runtime(self.rt_ptr).unwrap();
        }
        std::mem::forget(self.context);
        std::mem::forget(self.runtime);
    }
}

#[test]
fn function_template_constructor_has_one_bridge_owner() {
    let _lock = abi_test_lock();
    let mut fixture = WiredTestContext::new();

    let function_id = fixture.register_function_template();
    let root = fixture.get_function(function_id);

    assert_ne!(root, 0);
    assert_eq!(fixture.function_root(function_id), Some(root));

    let proto = fixture.template_prototype(function_id);
    let roots_before = fixture.bridge_strong_root_count();
    unsafe {
        assert_eq!(
            crate::bridge::prototype_has_own_constructor(fixture.ctx_ptr, proto).unwrap(),
            true
        );
        assert_eq!(fixture.bridge_strong_root_count(), roots_before);
    }

    unsafe {
        crate::shutdown_context(fixture.ctx_ptr).unwrap();
    }

    assert!(crate::bridge::teardown_counts_for_ctx(fixture.ctx_ptr).is_zero());

    fixture.shutdown_bridge_and_leak_runtime();
}

#[test]
fn prototype_install_does_not_leak_when_key_is_missing() {
    let _lock = abi_test_lock();
    let mut fixture = WiredTestContext::new();

    fixture.register_template_property_with_missing_key();
    let function_id = fixture.register_function_template();
    let _ = fixture.get_function(function_id);

    unsafe {
        crate::shutdown_context(fixture.ctx_ptr).unwrap();
    }

    assert!(crate::bridge::teardown_counts_for_ctx(fixture.ctx_ptr).is_zero());

    fixture.shutdown_bridge_and_leak_runtime();
}

#[test]
fn shutdown_context_releases_all_registered_root_kinds() {
    let _lock = abi_test_lock();
    let mut fixture = WiredTestContext::new();

    let function_id = fixture.register_function_template();
    let _ = fixture.get_function(function_id);

    crate::bridge::with_bridge_roots(|ctx, roots| {
        let obj = unsafe { rquickjs::qjs::JS_NewObject(ctx) };
        let root = roots.insert_borrowed(ctx, obj);
        unsafe { rquickjs::qjs::JS_FreeValue(ctx, obj) };
        roots.drop_root(ctx, root);
    });

    unsafe {
        crate::shutdown_context(fixture.ctx_ptr).unwrap();
    }

    assert!(crate::bridge::teardown_counts_for_ctx(fixture.ctx_ptr).is_zero());

    fixture.shutdown_bridge_and_leak_runtime();
}

#[test]
fn module_init_drops_temporary_roots_on_success() {
    let _lock = abi_test_lock();
    use std::ffi::c_void;

    unsafe extern "C" fn noop_register(
        _exports: *mut c_void,
        _module: *mut c_void,
        _priv_data: *mut c_void,
    ) {
    }

    let fixture = WiredTestContext::new();
    let exports = unsafe { rquickjs::qjs::JS_NewObject(fixture.ctx_ptr) };
    let mut module = crate::module_loader::NodeModule {
        nm_version: crate::NODE_MODULE_VERSION,
        nm_flags: 0,
        nm_dso_handle: std::ptr::null_mut(),
        nm_filename: std::ptr::null(),
        nm_register_func: Some(noop_register),
        nm_context_register_func: None,
        nm_modname: std::ptr::null(),
        nm_priv: std::ptr::null_mut(),
        nm_link: std::ptr::null_mut(),
    };

    let result =
        unsafe { crate::run_v8_module_init(fixture.ctx_ptr, &mut module, exports) }.unwrap();
    unsafe {
        rquickjs::qjs::JS_FreeValue(fixture.ctx_ptr, result);
    }

    unsafe {
        crate::shutdown_context(fixture.ctx_ptr).unwrap();
    }
    assert!(crate::bridge::teardown_counts_for_ctx(fixture.ctx_ptr).is_zero());
    fixture.shutdown_bridge_and_leak_runtime();
}

#[test]
fn module_init_drops_temporary_roots_on_failure() {
    let _lock = abi_test_lock();

    let fixture = WiredTestContext::new();
    let exports = unsafe { rquickjs::qjs::JS_NewObject(fixture.ctx_ptr) };
    let mut module = crate::module_loader::NodeModule {
        nm_version: crate::NODE_MODULE_VERSION,
        nm_flags: 0,
        nm_dso_handle: std::ptr::null_mut(),
        nm_filename: std::ptr::null(),
        nm_register_func: None,
        nm_context_register_func: None,
        nm_modname: std::ptr::null(),
        nm_priv: std::ptr::null_mut(),
        nm_link: std::ptr::null_mut(),
    };

    let result = unsafe { crate::run_v8_module_init(fixture.ctx_ptr, &mut module, exports) };
    assert!(result.is_err(), "module init should fail");
    unsafe {
        rquickjs::qjs::JS_FreeValue(fixture.ctx_ptr, exports);
    }
    let err = result.err().unwrap();
    assert!(err.contains("failed"), "unexpected: {err}");

    unsafe {
        crate::shutdown_context(fixture.ctx_ptr).unwrap();
    }
    assert!(crate::bridge::teardown_counts_for_ctx(fixture.ctx_ptr).is_zero());
    fixture.shutdown_bridge_and_leak_runtime();
}
