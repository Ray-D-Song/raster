// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    pin::{pin, Pin},
    ptr::NonNull,
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, MutexGuard,
    },
    time::Duration,
};

use once_cell::sync::Lazy;
use raster_runtime_context::CtxExtension;
use raster_runtime_hooking::allocate_hook_resource_id;
pub use raster_runtime_hooking::{invoke_async_hook, HookType};
use raster_runtime_utils::{
    module::{export_default, ModuleInfo},
    object::ObjectExt,
    provider::ProviderType,
};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Opt,
    qjs, Ctx, Exception, Function, Object, Persistent, Result, Value,
};
use tokio::{
    select,
    sync::Notify,
    time::{Instant, Sleep},
};

static TIMER_ID: AtomicUsize = AtomicUsize::new(0);
static RT_TIMER_STATE: Lazy<Mutex<Vec<RuntimeTimerState>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub struct RuntimeTimerState {
    timers: Vec<Timeout>,
    rt: *mut qjs::JSRuntime,
    running: bool,
    deadline: Instant,
    notify: Rc<Notify>,
    /// hook_id of the timer whose callback is currently executing (0 = none).
    /// Lets `clear_timeout_interval` detect a self-clear inside the callback
    /// and defer destroy until after After, matching Node's lifecycle order
    /// (init → before → after → destroy).
    executing_hook_id: usize,
}
impl RuntimeTimerState {
    fn new(rt: *mut qjs::JSRuntime) -> Self {
        let deadline = Instant::now() + Duration::from_secs(86400 * 365 * 30);
        Self {
            timers: Default::default(),
            rt,
            deadline,
            running: false,
            notify: Default::default(),
            executing_hook_id: 0,
        }
    }
}

unsafe impl Send for RuntimeTimerState {}

#[derive(Clone)]
pub struct Timeout {
    callback: Option<Persistent<Function<'static>>>,
    deadline: Instant,
    raw_ctx: NonNull<qjs::JSContext>,
    id: usize,
    hook_id: usize,
    repeating: bool,
    interval: u64,
    /// Set when the timer was cleared *inside its own callback*. The clear
    /// must not fire destroy immediately (that would drop the id_map entry
    /// before After runs and leak the async context stack); poll_timers
    /// fires destroy after After once this flag is set.
    pending_destroy: bool,
}

impl Default for Timeout {
    fn default() -> Self {
        Self {
            callback: None,
            deadline: Instant::now(),
            raw_ctx: NonNull::dangling(),
            id: 0,
            hook_id: 0,
            repeating: false,
            interval: 0,
            pending_destroy: false,
        }
    }
}

fn queue_microtask<'js>(ctx: Ctx<'js>, cb: Function<'js>) -> Result<()> {
    let uid = allocate_hook_resource_id();
    invoke_async_hook(&ctx, HookType::Init, ProviderType::Microtask, uid)?;

    // Stay on the true microtask queue (cb.defer); wrap for BEFORE/AFTER/destroy.
    let wrap: Function = ctx.eval(
        r#"(function (cb, uid) {
  return function () {
    var inv = globalThis.invokeAsyncHook;
    if (typeof inv === "function") {
      inv("before", "", uid);
      try {
        return cb();
      } finally {
        inv("after", "", uid);
        inv("destroy", "Microtask", uid);
      }
    }
    return cb();
  };
})"#,
    )?;
    let wrapped: Function = wrap.call((cb, uid as f64))?;
    wrapped.defer::<()>(())?;
    Ok(())
}

pub fn set_timeout_interval<'js>(
    ctx: &Ctx<'js>,
    cb: Function<'js>,
    delay: u64,
    provider_type: ProviderType,
) -> Result<usize> {
    // NOTE: https://noncodersuccess.medium.com/understanding-setimmediate-vs-settimeout-in-node-js-6a3ef8fc02d4
    // If `setImmediate(fn)` and `setTimeout(fn, 0) are queued at the exact same time,
    // `setImmediate(fn) takes precedence in Node.js, regardless of their execution order.
    let (repeating, deadline) = match provider_type {
        ProviderType::Immediate => (false, Instant::now() - Duration::from_secs(600)),
        ProviderType::Timeout => (false, Instant::now() + Duration::from_millis(delay)),
        ProviderType::Interval => (true, Instant::now() + Duration::from_millis(delay)),
        _ => {
            return Err(Exception::throw_type(
                ctx,
                "The specified provider type is not supported.",
            ))
        },
    };

    // External timer id (returned to JS, used by clearTimeout) stays a
    // per-module monotonic counter. The async-hooks lifecycle uses a
    // process-wide unique hook_resource_id so it cannot collide with
    // TickObject / Microtask entries in the shared id_map.
    let id = TIMER_ID.fetch_add(1, Ordering::Relaxed);
    let hook_id = allocate_hook_resource_id();
    invoke_async_hook(ctx, HookType::Init, provider_type, hook_id)?;

    let callback = Persistent::<Function>::save(ctx, cb);

    let timeout = Timeout {
        deadline,
        callback: Some(callback),
        raw_ctx: ctx.as_raw(),
        id,
        hook_id,
        repeating,
        interval: delay,
        pending_destroy: false,
    };

    let rt_ptr = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };

    let mut rt_timer = RT_TIMER_STATE.lock().unwrap();
    let state = get_timer_state(&mut rt_timer, rt_ptr);
    state.timers.push(timeout);
    let task_running = state.running;
    if task_running {
        if deadline < state.deadline {
            state.deadline = deadline;
            state.notify.notify_one();
        }
    } else {
        state.running = true;
        let timer_abort = state.notify.clone();
        drop(rt_timer);
        create_spawn_loop(rt_ptr, ctx, timer_abort, deadline)?;
    }

    Ok(id)
}

fn get_timer_state<'a>(
    state_ref: &'a mut MutexGuard<Vec<RuntimeTimerState>>,
    rt: *mut qjs::JSRuntime,
) -> &'a mut RuntimeTimerState {
    let rt_timers = state_ref.iter_mut().find(|state| state.rt == rt);

    //save a branch
    unsafe { rt_timers.unwrap_unchecked() }
}

fn clear_timeout_interval(ctx: Ctx<'_>, id: Opt<Value>) -> Result<()> {
    if let Some(id) = id.0.and_then(|v| v.as_number()) {
        let id = id as usize;
        let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
        let mut rt_timers = RT_TIMER_STATE.lock().unwrap();

        let state = get_timer_state(&mut rt_timers, rt);
        if let Some(timeout) = state.timers.iter_mut().find(|t| t.id == id) {
            let _ = timeout.callback.take();
            timeout.repeating = false;
            timeout.deadline = Instant::now() - Duration::from_secs(1);
            let hook_id = timeout.hook_id;
            // If this timer's own callback is currently executing, defer the
            // destroy until After has run and popped the async context
            // (otherwise After cannot find the id_map entry and the context
            // stack leaks). External clears fire destroy immediately.
            let self_clear = state.executing_hook_id == hook_id && hook_id != 0;
            if self_clear {
                timeout.pending_destroy = true;
                // Wake the timer loop so it re-polls now and drops the pending
                // entry, instead of sleeping until the next interval cycle
                // (which would delay process exit by a full period).
                state.notify.notify_one();
                drop(rt_timers);
                return Ok(());
            }
            state.notify.notify_one();
            // Drop async-id map entry for this timer resource.
            drop(rt_timers);
            if let Ok(Some(func)) = ctx.globals().get_optional::<_, Function>("invokeAsyncHook") {
                let _ = func.call::<_, ()>(("destroy", "Timeout", hook_id));
            }
            return Ok(());
        }
    }

    Ok(())
}

/// Install public timer globals with Node-compatible rest-arg forwarding and
/// `util.promisify.custom` on `setImmediate`.
const INSTALL_TIMER_WRAPPERS_JS: &str = r#"(function (nativeSetTimeout, nativeSetInterval, nativeSetImmediate, nativeClear, queueMicrotask) {
  function setTimeout(callback, delay, ...args) {
    if (typeof callback !== "function") {
      throw new TypeError('The "callback" argument must be of type function. Received type ' + typeof callback);
    }
    const ms = delay === undefined || delay === null ? 0 : Number(delay);
    return nativeSetTimeout(function () {
      callback(...args);
    }, ms);
  }

  function setInterval(callback, delay, ...args) {
    if (typeof callback !== "function") {
      throw new TypeError('The "callback" argument must be of type function. Received type ' + typeof callback);
    }
    const ms = delay === undefined || delay === null ? 0 : Number(delay);
    return nativeSetInterval(function () {
      callback(...args);
    }, ms);
  }

  function setImmediate(callback, ...args) {
    if (typeof callback !== "function") {
      throw new TypeError('The "callback" argument must be of type function. Received type ' + typeof callback);
    }
    return nativeSetImmediate(function () {
      callback(...args);
    });
  }

  // Next stores util.promisify(setImmediate) at startup. Provide custom so
  // promisify resolves a value without loading timers/promises (avoids cycles).
  const kCustom = Symbol.for("nodejs.util.promisify.custom");
  setImmediate[kCustom] = function setImmediatePromisify(value) {
    return new Promise((resolve) => {
      setImmediate(resolve, value);
    });
  };

  const clearTimeout = nativeClear;
  const clearInterval = nativeClear;
  const clearImmediate = nativeClear;

  globalThis.setTimeout = setTimeout;
  globalThis.setInterval = setInterval;
  globalThis.setImmediate = setImmediate;
  globalThis.clearTimeout = clearTimeout;
  globalThis.clearInterval = clearInterval;
  globalThis.clearImmediate = clearImmediate;
  globalThis.queueMicrotask = queueMicrotask;

  return {
    setTimeout,
    setInterval,
    setImmediate,
    clearTimeout,
    clearInterval,
    clearImmediate,
    queueMicrotask,
  };
})"#;

const TIMERS_PROMISES_FACTORY_JS: &str = r#"(function () {
  function validateOptions(options, name) {
    if (options === undefined || options === null) {
      return {};
    }
    if (typeof options !== "object") {
      throw new TypeError(
        'The "' + name + '" argument must be of type object. Received type ' + typeof options
      );
    }
    if (options.signal !== undefined && options.signal !== null) {
      const signal = options.signal;
      if (typeof signal !== "object" || typeof signal.aborted !== "boolean") {
        throw new TypeError('The "options.signal" property must be an AbortSignal.');
      }
    }
    if (options.ref !== undefined && typeof options.ref !== "boolean") {
      throw new TypeError('The "options.ref" property must be of type boolean.');
    }
    return options;
  }

  function setTimeout(delay, value, options) {
    try {
      options = validateOptions(options, "options");
    } catch (err) {
      return Promise.reject(err);
    }
    const signal = options.signal;
    // { ref: false } is accepted for API compatibility; Raster does not yet
    // change event-loop lifetime based on timer handles.
    if (signal && signal.aborted) {
      return Promise.reject(signal.reason);
    }
    const ms = delay === undefined || delay === null ? 0 : Number(delay);
    return new Promise((resolve, reject) => {
      let settled = false;
      const onAbort = () => {
        if (settled) return;
        settled = true;
        clearTimeout(id);
        signal.removeEventListener("abort", onAbort);
        reject(signal.reason);
      };
      const id = globalThis.setTimeout(() => {
        if (settled) return;
        settled = true;
        if (signal) {
          signal.removeEventListener("abort", onAbort);
        }
        resolve(value);
      }, ms);
      if (signal) {
        signal.addEventListener("abort", onAbort);
      }
    });
  }

  function setImmediate(value, options) {
    try {
      options = validateOptions(options, "options");
    } catch (err) {
      return Promise.reject(err);
    }
    const signal = options.signal;
    if (signal && signal.aborted) {
      return Promise.reject(signal.reason);
    }
    return new Promise((resolve, reject) => {
      let settled = false;
      const onAbort = () => {
        if (settled) return;
        settled = true;
        clearImmediate(id);
        signal.removeEventListener("abort", onAbort);
        reject(signal.reason);
      };
      const id = globalThis.setImmediate(() => {
        if (settled) return;
        settled = true;
        if (signal) {
          signal.removeEventListener("abort", onAbort);
        }
        resolve(value);
      });
      if (signal) {
        signal.addEventListener("abort", onAbort);
      }
    });
  }

  // Extensible CJS export object so Next can assign patched setImmediate.
  return {
    setTimeout,
    setImmediate,
  };
})"#;

pub struct TimersModule;

impl ModuleDef for TimersModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("setTimeout")?;
        declare.declare("clearTimeout")?;
        declare.declare("setInterval")?;
        declare.declare("clearInterval")?;
        declare.declare("setImmediate")?;
        declare.declare("clearImmediate")?;
        declare.declare("queueMicrotask")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let globals = ctx.globals();

        export_default(ctx, exports, |default| {
            let functions = [
                "setTimeout",
                "clearTimeout",
                "setInterval",
                "clearInterval",
                "setImmediate",
                "clearImmediate",
                "queueMicrotask",
            ];
            for func_name in functions {
                let function: Function = globals.get(func_name)?;
                default.set(func_name, function)?;
            }
            Ok(())
        })?;

        Ok(())
    }
}

impl From<TimersModule> for ModuleInfo<TimersModule> {
    fn from(val: TimersModule) -> Self {
        ModuleInfo {
            name: "timers",
            module: val,
        }
    }
}

/// `node:timers/promises` — promise-based timers reusing global schedulers.
pub struct TimersPromisesModule;

impl ModuleDef for TimersPromisesModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("setTimeout")?;
        declare.declare("setImmediate")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let factory: Function = ctx.eval(TIMERS_PROMISES_FACTORY_JS)?;
        let api: Object = factory.call(())?;

        // Export a single mutable object for both named and default so CJS
        // assignment (Next patches setImmediate) works without freezing.
        let set_timeout: Function = api.get("setTimeout")?;
        let set_immediate: Function = api.get("setImmediate")?;

        exports.export("setTimeout", set_timeout.clone())?;
        exports.export("setImmediate", set_immediate.clone())?;
        exports.export("default", api)?;
        Ok(())
    }
}

impl From<TimersPromisesModule> for ModuleInfo<TimersPromisesModule> {
    fn from(val: TimersPromisesModule) -> Self {
        ModuleInfo {
            name: "timers/promises",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let rt_ptr = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };

    let mut rt_timers = RT_TIMER_STATE.lock().unwrap();
    rt_timers.push(RuntimeTimerState::new(rt_ptr));
    drop(rt_timers);

    let native_set_timeout = Function::new(ctx.clone(), move |ctx, cb, delay: Opt<f64>| {
        let delay = delay.unwrap_or(0.).max(0.) as u64;
        set_timeout_interval(&ctx, cb, delay, ProviderType::Timeout)
    })?;
    let native_set_interval = Function::new(ctx.clone(), move |ctx, cb, delay: Opt<f64>| {
        let delay = delay.unwrap_or(0.).max(0.) as u64;
        set_timeout_interval(&ctx, cb, delay, ProviderType::Interval)
    })?;
    let native_set_immediate = Function::new(ctx.clone(), move |ctx, cb| {
        set_timeout_interval(&ctx, cb, 0, ProviderType::Immediate)
    })?;
    let native_clear = Function::new(ctx.clone(), clear_timeout_interval)?;
    let native_queue_microtask = Function::new(ctx.clone(), queue_microtask)?;

    let install: Function = ctx.eval(INSTALL_TIMER_WRAPPERS_JS)?;
    let _: Object = install.call((
        native_set_timeout,
        native_set_interval,
        native_set_immediate,
        native_clear,
        native_queue_microtask,
    ))?;

    Ok(())
}

#[inline(always)]
fn create_spawn_loop(
    rt: *mut qjs::JSRuntime,
    ctx: &Ctx<'_>,
    timer_abort: Rc<Notify>,
    deadline: Instant,
) -> Result<()> {
    ctx.spawn_exit_simple(async move {
        let mut sleep = pin!(tokio::time::sleep_until(deadline));

        let mut executing_timers: Vec<Option<ExecutingTimer>> = Default::default();

        loop {
            select! {
                _ = timer_abort.notified() => {}
                _ = sleep.as_mut() => {}
            }

            if !poll_timers(rt, &mut executing_timers, Some(&mut sleep), None)? {
                break;
            }
        }
        Ok(())
    });

    Ok(())
}

pub struct ExecutingTimer(
    Instant,
    NonNull<qjs::JSContext>,
    Persistent<Function<'static>>,
    usize, // hook resource id for async hooks (not the external timer id)
);

unsafe impl Send for ExecutingTimer {}

/// RAII guard that clears `executing_hook_id` on drop so an early `?`
/// return from Before/After/callback cannot leave the marker stale.
struct ExecutingGuard {
    rt: *mut qjs::JSRuntime,
}
impl Drop for ExecutingGuard {
    fn drop(&mut self) {
        if let Ok(mut rt_timers) = RT_TIMER_STATE.lock() {
            let state = get_timer_state(&mut rt_timers, self.rt);
            state.executing_hook_id = 0;
        }
    }
}

pub fn poll_timers(
    rt: *mut qjs::JSRuntime,
    call_vec: &mut Vec<Option<ExecutingTimer>>,
    sleep: Option<&mut Pin<&mut Sleep>>,
    deadline: Option<&mut Instant>,
) -> Result<bool> {
    static MIN_SLEEP: Duration = Duration::from_millis(4);
    static FAR_FUTURE: Duration = Duration::from_secs(84200 * 365 * 30);

    let mut rt_timers = RT_TIMER_STATE.lock().unwrap();
    let state = get_timer_state(&mut rt_timers, rt);
    let now = Instant::now();

    let mut had_items = false;
    let mut lowest = now + FAR_FUTURE;
    state.timers.retain_mut(|timeout| {
        had_items = true;
        if timeout.deadline < now {
            let ctx = timeout.raw_ctx;
            if let Some(cb) = timeout.callback.take() {
                if !timeout.repeating {
                    call_vec.push(Some(ExecutingTimer(
                        timeout.deadline,
                        ctx,
                        cb,
                        timeout.hook_id,
                    )));
                    return false;
                }
                timeout.deadline = now + Duration::from_millis(timeout.interval);
                if timeout.deadline < lowest {
                    lowest = timeout.deadline;
                }
                call_vec.push(Some(ExecutingTimer(
                    timeout.deadline,
                    ctx,
                    cb.clone(),
                    timeout.hook_id,
                )));
                timeout.callback.replace(cb);
            } else {
                return false;
            }
        } else if timeout.deadline < lowest {
            lowest = timeout.deadline;
        }
        true
    });

    let has_items = !state.timers.is_empty();

    if had_items {
        if lowest - now < MIN_SLEEP {
            lowest = now + MIN_SLEEP;
        }
        if let Some(sleep) = sleep {
            sleep.as_mut().reset(lowest);
        }
        if let Some(deadline) = deadline {
            *deadline = lowest;
        }
        state.deadline = lowest;
    }

    drop(rt_timers);

    call_vec.sort_unstable_by_key(|v| v.as_ref().map(|v| v.0));

    let mut is_first_time = true;
    for item in call_vec.iter_mut() {
        if let Some(ExecutingTimer(_, ctx, timeout, hook_id)) = item.take() {
            let ctx2 = unsafe { Ctx::from_raw(ctx) };

            if is_first_time {
                while ctx2.execute_pending_job() {}
                is_first_time = false;
            }

            {
                let _guard = ExecutingGuard { rt };
                // Mark this timer as executing so a self-clearInterval inside
                // the callback defers destroy until after After.
                {
                    let mut rt_timers = RT_TIMER_STATE.lock().unwrap();
                    get_timer_state(&mut rt_timers, rt).executing_hook_id = hook_id;
                }

                if let Ok(timeout) = timeout.restore(&ctx2) {
                    invoke_async_hook(&ctx2, HookType::Before, ProviderType::None, hook_id)?;

                    // User callback errors must not leave async context dirty.
                    let call_result = timeout.call::<_, ()>(());

                    invoke_async_hook(&ctx2, HookType::After, ProviderType::None, hook_id)?;

                    // Destroy after After when the timer left the list
                    // (one-shot completed) or was cleared inside its own
                    // callback (pending_destroy). Intervals that persist keep
                    // their id_map entry across fires.
                    let destroy_now = {
                        let mut rt_timers = RT_TIMER_STATE.lock().unwrap();
                        let state = get_timer_state(&mut rt_timers, rt);
                        match state.timers.iter().find(|t| t.hook_id == hook_id) {
                            None => true,
                            Some(t) => t.pending_destroy,
                        }
                    };
                    if destroy_now {
                        if let Ok(Some(func)) = ctx2
                            .globals()
                            .get_optional::<_, Function>("invokeAsyncHook")
                        {
                            let _ = func.call::<_, ()>(("destroy", "Timeout", hook_id));
                        }
                    }

                    call_result?;
                }
            }
            // _guard dropped: executing_hook_id cleared before pending jobs.

            while ctx2.execute_pending_job() {}
        }
    }
    call_vec.clear();

    if !has_items {
        let mut rt_timers = RT_TIMER_STATE.lock().unwrap();
        let state = get_timer_state(&mut rt_timers, rt);
        let is_empty = state.timers.is_empty();
        state.running = !is_empty;

        return Ok(!is_empty);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use raster_runtime_test::{call_test, test_async_with, ModuleEvaluator};

    use super::*;

    #[tokio::test]
    async fn test_timers() {
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<TimersModule>(ctx.clone(), "timers")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_setTimeout",
                    r#"
                        import { setTimeout } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                setTimeout(() => resolve('timeout'), 100);
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "timeout");

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_setImmediate",
                    r#"
                        import { setImmediate } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                setImmediate(() => resolve('immediate'));
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "immediate");

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_args",
                    r#"
                        import { setTimeout, setImmediate } from 'timers';
                        export async function test() {
                            const a = await new Promise((resolve) => {
                                setTimeout((x, y) => resolve(x + y), 10, 1, 2);
                            });
                            const b = await new Promise((resolve) => {
                                setImmediate((x) => resolve(x), 'ok');
                            });
                            return [a, b];
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<Vec<rquickjs::Value>, _>(&ctx, &module, ()).await;
                // Just ensure it completes without hang; detailed checks in JS unit tests.
                assert_eq!(result.len(), 2);

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_clearImmediate",
                    r#"
                        import { setImmediate, clearImmediate } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                const id = setImmediate(() => resolve('should not'));
                                clearImmediate(id);
                                setImmediate(() => resolve('canceled'));
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "canceled");
            })
        })
        .await;
    }

    /// Regression: an interval cleared inside its own callback must wake the
    /// timer loop so the pending entry is dropped promptly (within a few ms),
    /// not after a full interval cycle. Without the wake, the loop sleeps
    /// until the next period and a CLI process cannot exit until then.
    ///
    /// The check inspects the internal timer list directly from Rust because
    /// any JS-side probe timer would itself notify the loop and mask the bug.
    #[tokio::test]
    async fn self_clearing_interval_wakes_loop_and_drops_entry() {
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<TimersModule>(ctx.clone(), "timers")
                    .await
                    .unwrap();

                let rt_ptr = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };

                // Schedule a self-clearing interval. The JS fn resolves the
                // promise from inside the callback, so once `call_test`
                // returns we know the clear already happened. No other timer
                // is scheduled, so nothing masks a missing loop wake-up.
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_self_clear_wake",
                    r#"
                        export function test() {
                            return new Promise((resolve) => {
                                const handle = setInterval(() => {
                                    clearInterval(handle);
                                    resolve();
                                }, 300);
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                call_test::<(), _>(&ctx, &module, ()).await;

                // With the fix the loop is woken during the await above and the
                // pending entry is already gone. Poll for a window well short
                // of the next 300ms cycle to also cover slow CI scheduling.
                let mut removed = false;
                for _ in 0..15 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    let empty = {
                        let rt_timers = RT_TIMER_STATE.lock().unwrap();
                        rt_timers
                            .iter()
                            .find(|s| s.rt == rt_ptr)
                            .map(|s| s.timers.is_empty())
                            .unwrap_or(true)
                    };
                    if empty {
                        removed = true;
                        break;
                    }
                }
                assert!(
                    removed,
                    "timer loop was not woken after interval self-clear; \
                     pending entry lingered for a full cycle"
                );
            })
        })
        .await;
    }
}
