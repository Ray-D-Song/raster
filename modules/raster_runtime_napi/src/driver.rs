// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use libc::pthread_t;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;

use parking_lot::Mutex;
use raster_runtime_context::CtxExtension;
use rquickjs::qjs::JSRuntime;
use rquickjs::Ctx;
use tokio::sync::mpsc;
use tokio::sync::Mutex as AsyncMutex;

use crate::env::Env;
use crate::types::{napi_env, napi_status};

pub enum DriverJob {
    AsyncComplete {
        work: Arc<crate::async_work::AsyncWorkState>,
        status: napi_status,
    },
    Tsfn {
        tsfn: Arc<crate::async_work::ThreadsafeFunction>,
        done: Option<SyncSender<()>>,
    },
    TsfnRelease {
        tsfn: Arc<crate::async_work::ThreadsafeFunction>,
    },
    TsfnAbort {
        tsfn: Arc<crate::async_work::ThreadsafeFunction>,
    },
    /// Wakes a parked `run_loop` when it should observe quiescence and exit.
    Wake,
}

pub struct DriverState {
    tx: Mutex<Option<mpsc::UnboundedSender<DriverJob>>>,
    rx: AsyncMutex<mpsc::UnboundedReceiver<DriverJob>>,
    pub pending: AtomicUsize,
    pub idle_refs: AtomicUsize,
    /// In-flight async work items that must keep the driver loop alive until complete.
    pub inflight_async: AtomicUsize,
    loop_running: AtomicBool,
    pub js_pthread: pthread_t,
    pub runtime: tokio::runtime::Handle,
    pub rt_ptr: *mut JSRuntime,
    #[cfg(test)]
    fail_posts: AtomicBool,
}

unsafe impl Send for DriverState {}
unsafe impl Sync for DriverState {}

impl DriverState {
    pub fn new(js_pthread: pthread_t, rt_ptr: *mut JSRuntime) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx: Mutex::new(Some(tx)),
            rx: AsyncMutex::new(rx),
            pending: AtomicUsize::new(0),
            idle_refs: AtomicUsize::new(0),
            inflight_async: AtomicUsize::new(0),
            loop_running: AtomicBool::new(false),
            js_pthread,
            runtime: tokio::runtime::Handle::current(),
            rt_ptr,
            #[cfg(test)]
            fail_posts: AtomicBool::new(false),
        }
    }

    #[cfg(test)]
    pub fn set_fail_posts(&self, fail: bool) {
        self.fail_posts.store(fail, Ordering::SeqCst);
    }

    /// Thread-safe: enqueue a job without touching `Env` or QuickJS.
    pub fn post_job(&self, job: DriverJob) -> bool {
        #[cfg(test)]
        if self.fail_posts.load(Ordering::SeqCst) {
            return false;
        }
        let guard = self.tx.lock();
        let Some(tx) = guard.as_ref() else {
            return false;
        };
        self.pending.fetch_add(1, Ordering::SeqCst);
        if tx.send(job).is_err() {
            drop(guard);
            self.dec_pending();
            return false;
        }
        drop(guard);
        if !self.rt_ptr.is_null() {
            raster_runtime_utils::driver_poll::wake_native_drivers(self.rt_ptr);
        }
        true
    }

    pub fn acquire_async_keepalive(&self) {
        self.inflight_async.fetch_add(1, Ordering::SeqCst);
    }

    pub fn release_async_keepalive(&self) {
        let _ = self
            .inflight_async
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                if v == 0 {
                    None
                } else {
                    Some(v - 1)
                }
            });
        self.wake_if_quiescent();
    }

    pub fn should_keep_loop_alive(&self) -> bool {
        self.pending.load(Ordering::Relaxed) > 0
            || self.idle_refs.load(Ordering::Relaxed) > 0
            || self.inflight_async.load(Ordering::Relaxed) > 0
    }

    pub fn pending_count(&self) -> usize {
        self.pending.load(Ordering::SeqCst)
    }

    /// If the driver loop is parked on an empty queue with no ref'd work, nudge it
    /// so it can observe the new quiescent state and exit.
    pub fn wake_if_quiescent(&self) {
        if !self.should_keep_loop_alive() {
            self.post_job(DriverJob::Wake);
        }
    }

    /// JS-thread only: enqueue and ensure the driver loop is running.
    pub fn post(&self, env: &mut Env, job: DriverJob) -> bool {
        if !self.post_job(job) {
            return false;
        }
        self.ensure_loop(env);
        true
    }

    pub fn should_ensure_loop(&self) -> bool {
        self.pending.load(Ordering::SeqCst) > 0 && !self.loop_running.load(Ordering::SeqCst)
    }

    pub fn ensure_loop(&self, env: &mut Env) {
        let driver = env.driver.clone().expect("driver");
        driver.spawn_loop(env as *mut Env as usize, env.ctx_ptr());
    }

    fn spawn_loop(self: &Arc<Self>, env_addr: usize, ctx_ptr: *mut rquickjs::qjs::JSContext) {
        if self.loop_running.load(Ordering::SeqCst) {
            return;
        }
        let on_js_thread =
            unsafe { libc::pthread_equal(libc::pthread_self(), self.js_pthread) != 0 };
        if !on_js_thread {
            debug_assert!(
                on_js_thread,
                "spawn_loop must run on the Env's registered JS thread"
            );
            tracing::debug!("spawn_loop skipped: current pthread does not match Env JS thread");
            return;
        }
        if self.loop_running.swap(true, Ordering::SeqCst) {
            return;
        }
        let driver = Arc::clone(self);
        let ctx = unsafe { Ctx::from_raw(NonNull::new_unchecked(ctx_ptr)) };
        ctx.spawn_exit_simple(async move {
            driver.run_loop(env_addr).await;
            Ok(())
        });
    }

    async fn run_loop(self: Arc<Self>, env_addr: usize) {
        let _napi_env = env_addr as napi_env;
        loop {
            if !self.should_keep_loop_alive() {
                let mut rx = self.rx.lock().await;
                match rx.try_recv() {
                    Ok(job) => {
                        drop(rx);
                        self.dispatch_job(env_addr, job);
                        continue;
                    },
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => break,
                }
            }

            let job = self.rx.lock().await.recv().await;
            let Some(job) = job else {
                break;
            };
            self.dispatch_job(env_addr, job);
        }
        self.loop_running.store(false, Ordering::SeqCst);
    }

    fn dispatch_job(&self, env_addr: usize, job: DriverJob) {
        unsafe {
            crate::async_work::process_driver_job(env_addr as *mut Env, job);
        }
        self.dec_pending();
        unsafe {
            crate::gc_hook::drain_pending_finalizers(&mut *(env_addr as *mut Env));
        }
    }

    pub fn sender_clone(&self) -> Option<mpsc::UnboundedSender<DriverJob>> {
        self.tx.lock().clone()
    }

    pub fn close_sender(&self) {
        self.tx.lock().take();
    }

    fn dec_pending(&self) {
        let _ = self
            .pending
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                Some(v.saturating_sub(1))
            });
    }

    #[cfg(test)]
    fn drain_jobs_for_test(&self) {
        let Ok(mut rx) = self.rx.try_lock() else {
            return;
        };
        while rx.try_recv().is_ok() {
            self.dec_pending();
        }
    }

    /// JS-thread only: process any jobs already in the driver queue.
    pub fn drain_ready_jobs(&self, env: &mut Env) {
        let env_ptr = env as *mut Env;
        let Ok(mut rx) = self.rx.try_lock() else {
            return;
        };
        loop {
            match rx.try_recv() {
                Ok(job) => {
                    unsafe {
                        crate::async_work::process_driver_job(env_ptr, job);
                    }
                    self.dec_pending();
                    crate::gc_hook::drain_pending_finalizers(env);
                },
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => break,
            }
        }
    }
}

pub fn ensure_driver(env: &mut Env) -> Arc<DriverState> {
    if env.driver.is_none() {
        let rt_ptr = unsafe { rquickjs::qjs::JS_GetRuntime(env.ctx_ptr()) };
        raster_runtime_utils::driver_poll::driver_notify_for_rt(rt_ptr);
        env.driver = Some(Arc::new(DriverState::new(env.js_pthread, rt_ptr)));
    }
    env.driver.clone().unwrap()
}

pub fn shutdown_driver(env: &mut Env) {
    if let Some(driver) = env.driver.take() {
        driver.close_sender();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Barrier;
    use std::thread;

    use crate::async_work::AsyncWorkState;
    use crate::types::napi_status;
    use std::sync::atomic::AtomicU8;

    #[test]
    fn post_job_pending_reaches_zero_after_concurrent_dispatch() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let driver = Arc::new(DriverState::new(
                unsafe { libc::pthread_self() },
                std::ptr::null_mut(),
            ));
            let work = Arc::new(AsyncWorkState {
                env: std::ptr::null_mut(),
                execute: None,
                complete: None,
                data: std::ptr::null_mut(),
                status: AtomicU8::new(0),
                completion_posted: AtomicBool::new(false),
            });
            let barrier = Arc::new(Barrier::new(3));
            let mut handles = Vec::new();
            for _ in 0..2 {
                let driver = Arc::clone(&driver);
                let work = Arc::clone(&work);
                let barrier = Arc::clone(&barrier);
                handles.push(thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..50 {
                        let _ = driver.post_job(DriverJob::AsyncComplete {
                            work: Arc::clone(&work),
                            status: napi_status::napi_ok,
                        });
                    }
                }));
            }
            barrier.wait();
            while driver.pending_count() > 0 {
                driver.drain_jobs_for_test();
                thread::yield_now();
            }
            for handle in handles {
                handle.join().unwrap();
            }
            driver.drain_jobs_for_test();
            assert_eq!(driver.pending_count(), 0);
        });
    }
}
