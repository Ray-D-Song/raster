// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Node-compatible process shutdown: drain the event loop, emit `beforeExit`,
//! drain again, then emit `exit` — the process only leaves through an explicit
//! `process.exit()`/signal default, never through `std::process::exit` here.
//!
//! One turn of the loop is: run pending JS work (`Vm::idle`), poll native
//! drivers, dispatch queued OS signals, then re-check whether anything still
//! keeps the process alive.

use std::sync::atomic::Ordering;

use raster_runtime_core::vm::Vm;
use raster_runtime_process::lifecycle::{
    default_exit_code_for_signal, emit_before_exit, emit_os_signal, emit_process_exit,
    signal_listener_count,
};
use raster_runtime_process::EXIT_CODE;
use raster_runtime_utils::{driver_poll, event_loop::quiescence_watch};
use rquickjs::{qjs, Ctx, Object};
use tokio::time::{sleep, Duration};

#[cfg(not(target_family = "wasm"))]
use crate::signal_bridge;

/// Set when a signal arrived without a JS listener: Node terminates with
/// `128 + signum` after emitting `exit`.
static SIGNAL_EXIT_REQUESTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// How long a drain turn waits on `Vm::idle` before re-checking liveness.
const DRAIN_POLL_INTERVAL: Duration = Duration::from_millis(20);

pub async fn run_until_process_exit(
    vm: &Vm,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rt_key = vm.js_runtime_key();

    loop {
        if schedule_signals(vm).await? {
            continue;
        }

        idle_or_signal(vm).await?;
        poll_drivers(vm).await;

        if schedule_signals(vm).await? {
            continue;
        }

        if SIGNAL_EXIT_REQUESTED.load(Ordering::SeqCst) {
            break;
        }

        if stay_alive(vm, rt_key).await? {
            wait_for_loop_wake(rt_key).await;
            continue;
        }

        vm.ctx
            .with(|ctx| emit_before_exit(&ctx))
            .await
            .map_err(|e: rquickjs::Error| e.to_string())?;

        idle_or_signal(vm).await?;
        poll_drivers(vm).await;

        if schedule_signals(vm).await? {
            continue;
        }

        if SIGNAL_EXIT_REQUESTED.load(Ordering::SeqCst) {
            break;
        }

        if stay_alive(vm, rt_key).await? {
            wait_for_loop_wake(rt_key).await;
            continue;
        }

        vm.ctx
            .with(|ctx| {
                let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
                raster_runtime_timers::discard_unrefed_timers_on_process_exit(&ctx, rt)
            })
            .await
            .map_err(|e: rquickjs::Error| e.to_string())?;

        idle_or_signal(vm).await?;

        vm.ctx
            .with(|ctx| emit_process_exit(&ctx))
            .await
            .map_err(|e: rquickjs::Error| e.to_string())?;

        break;
    }

    Ok(())
}

/// Run the QuickJS event loop until either it settles, an OS signal is queued,
/// or the (short) poll interval elapses.
///
/// `idle` parks as long as any spawned task is still pending — including tasks
/// that are merely waiting on another handle — so the drain loop must be able to
/// interrupt it to re-check whether the process can exit.
async fn idle_or_signal(vm: &Vm) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let idle = async {
        vm.idle().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };

    #[cfg(target_family = "wasm")]
    {
        idle.await?;
        return Ok(());
    }

    #[cfg(not(target_family = "wasm"))]
    {
        if signal_bridge::has_pending_signal() {
            return Ok(());
        }

        let wake = signal_bridge::wake_handle();
        tokio::select! {
            res = idle => res?,
            _ = wake.notified() => {},
            _ = sleep(DRAIN_POLL_INTERVAL) => {},
        }

        Ok(())
    }
}

/// Queue JS tasks for signals that arrived since the last turn.
///
/// The tasks are spawned into the runtime so they run inside `idle`, on the JS
/// thread and with the module's context — signal streams itself must not touch
/// QuickJS.
async fn schedule_signals(vm: &Vm) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(target_family = "wasm")]
    {
        let _ = vm;
        return Ok(false);
    }

    #[cfg(not(target_family = "wasm"))]
    {
        if !signal_bridge::has_pending_signal() {
            return Ok(false);
        }

        vm.ctx
            .with(|ctx| -> rquickjs::Result<()> {
                for name in signal_bridge::take_pending_signals() {
                    let task_ctx = ctx.clone();
                    let run_ctx = task_ctx.clone();
                    task_ctx.spawn(async move {
                        if let Err(err) = dispatch_signal(&run_ctx, &name) {
                            report_signal_error(&run_ctx, err);
                        }
                    });
                }
                Ok(())
            })
            .await
            .map_err(|e: rquickjs::Error| e.to_string())?;

        Ok(true)
    }
}

/// Emit `process` `"signal"`; without a listener request the default
/// termination (`128 + signum`) instead of exiting from inside the runtime.
fn dispatch_signal(ctx: &Ctx<'_>, name: &str) -> rquickjs::Result<()> {
    if emit_os_signal(ctx, name)? {
        return Ok(());
    }

    let code = default_exit_code_for_signal(name);
    EXIT_CODE.store(code, Ordering::Relaxed);
    emit_process_exit(ctx)?;
    // The runtime lock is held here, so unwinding is the drain loop's job.
    SIGNAL_EXIT_REQUESTED.store(true, Ordering::SeqCst);
    Ok(())
}

fn report_signal_error(ctx: &Ctx<'_>, err: rquickjs::Error) {
    let caught = ctx.catch();
    if let Some(exception) = caught.as_exception() {
        eprintln!("{}", exception.message().unwrap_or_default());
        if let Some(stack) = exception.stack() {
            eprintln!("{stack}");
        }
        return;
    }
    let value = caught.as_value();
    let text: std::result::Result<String, rquickjs::Error> = value.get();
    eprintln!("{}", text.unwrap_or_else(|_| format!("{err:?}")));
}

async fn poll_drivers(vm: &Vm) {
    let _ = vm
        .ctx
        .with(|ctx| {
            let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
            raster_runtime_timers::resume_timer_loop_if_idle(&ctx)?;
            driver_poll::poll_native_drivers(rt);
            Ok::<_, rquickjs::Error>(())
        })
        .await;
}

async fn stay_alive(vm: &Vm, rt_key: usize) -> Result<bool, String> {
    vm.ctx
        .with(|ctx| {
            let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
            if event_loop_alive(rt_key, rt) {
                return Ok(true);
            }
            // Node keeps the loop alive while a signal listener is installed
            // (the signal watcher is a referenced handle).
            if let Ok(process) = ctx.globals().get::<_, Object>("process") {
                if signal_listener_count(&ctx, &process)? > 0 {
                    return Ok(true);
                }
            }
            Ok(false)
        })
        .await
        .map_err(|e: rquickjs::Error| e.to_string())
}

/// Whether the JS loop still has work that must finish before `exit`:
/// live handle references or a referenced timer the scheduler is parked on.
fn event_loop_alive(rt_key: usize, rt: *mut qjs::JSRuntime) -> bool {
    raster_runtime_utils::event_loop::should_keep_process_alive(rt_key)
        || raster_runtime_timers::keeps_timers_alive(rt)
}

async fn wait_for_loop_wake(rt_key: usize) {
    let quiescent = quiescence_watch(rt_key);
    tokio::select! {
        _ = quiescent.notified() => {},
        _ = sleep(Duration::from_millis(50)) => {},
    }
}
