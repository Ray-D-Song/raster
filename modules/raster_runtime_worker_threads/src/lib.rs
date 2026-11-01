// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    env,
    io::Write,
    path::PathBuf,
    process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use raster_runtime_context::CtxExtension;
use raster_runtime_json::parse::json_parse;
use raster_runtime_utils::{
    event_loop::{self, EventLoopRef},
    module::{export_default, ModuleInfo},
    result::ResultExt,
};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    CatchResultExt, Ctx, Exception, Function, Object, Promise, Result, Undefined, Value,
};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use url::Url;

pub const ENV_WORKER_INTERNAL: &str = "RASTER_WORKER_INTERNAL";
pub const ENV_WORKER_SCRIPT: &str = "RASTER_WORKER_SCRIPT";
pub const ENV_WORKER_DATA: &str = "RASTER_WORKER_DATA";

static NEXT_THREAD_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum IpcMessage {
    Online,
    Message { data: serde_json::Value },
    Error { message: String },
    Exit { code: i32 },
}

struct WorkerState {
    stdin_tx: mpsc::UnboundedSender<IpcMessage>,
    loop_ref: Arc<EventLoopRef>,
    child: Arc<tokio::sync::Mutex<Option<tokio::process::Child>>>,
}

pub fn worker_is_main_thread(_ctx: Ctx<'_>) -> bool {
    env::var(ENV_WORKER_INTERNAL).is_err()
}

fn worker_thread_id(_ctx: Ctx<'_>) -> u64 {
    if worker_is_main_thread(_ctx.clone()) {
        0
    } else {
        env::var("RASTER_WORKER_THREAD_ID")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }
}

fn write_ipc_line(writer: &mut impl Write, msg: &IpcMessage) -> std::io::Result<()> {
    let line = serde_json::to_string(msg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    writeln!(writer, "{line}")?;
    writer.flush()
}

fn js_from_json<'js>(ctx: &Ctx<'js>, value: &serde_json::Value) -> Result<Value<'js>> {
    let s = serde_json::to_string(value).or_throw(ctx)?;
    json_parse(ctx, s.into_bytes())
}

fn resolve_worker_filename(ctx: &Ctx<'_>, filename: String) -> Result<String> {
    if let Some(path) = filename.strip_prefix("file://") {
        return Ok(path.to_string());
    }
    let path = PathBuf::from(&filename);
    if path.is_absolute() {
        return Ok(filename);
    }
    let cwd = env::current_dir().or_throw(ctx)?;
    Ok(cwd.join(path).to_string_lossy().into_owned())
}

fn spawn_worker<'js>(
    ctx: Ctx<'js>,
    filename: String,
    worker_data_json: String,
    on_event: Function<'js>,
) -> Result<Object<'js>> {
    let script = resolve_worker_filename(&ctx, filename)?;
    let data_json: serde_json::Value =
        serde_json::from_str(&worker_data_json).unwrap_or(serde_json::Value::Null);

    let exe = env::current_exe().or_throw(&ctx)?;
    let thread_id = NEXT_THREAD_ID.fetch_add(1, Ordering::Relaxed);

    let mut child = tokio::process::Command::new(exe)
        .env(ENV_WORKER_INTERNAL, "1")
        .env(ENV_WORKER_SCRIPT, &script)
        .env(
            ENV_WORKER_DATA,
            serde_json::to_string(&data_json).unwrap_or_else(|_| "null".into()),
        )
        .env("RASTER_WORKER_THREAD_ID", thread_id.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .or_throw(&ctx)?;

    let stdin = child.stdin.take().or_throw(&ctx)?;
    let stdout = child.stdout.take().or_throw(&ctx)?;

    let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<IpcMessage>();
    let loop_ref = EventLoopRef::new_referenced(event_loop::rt_key_from_ptr(unsafe {
        rquickjs::qjs::JS_GetRuntime(ctx.as_raw().as_ptr())
    }
        as *mut std::ffi::c_void));
    let state = Arc::new(WorkerState {
        stdin_tx,
        loop_ref: loop_ref.clone(),
        child: Arc::new(tokio::sync::Mutex::new(Some(child))),
    });

    let out = Object::new(ctx.clone())?;
    out.set("threadId", thread_id)?;

    let state_post = state.clone();
    out.set(
        "postMessage",
        Func::from(move |ctx: Ctx<'_>, data: String| {
            let json: serde_json::Value = serde_json::from_str(&data).or_throw(&ctx)?;
            let _ = state_post.stdin_tx.send(IpcMessage::Message { data: json });
            Ok::<(), rquickjs::Error>(())
        }),
    )?;

    let state_term = state.clone();
    out.set(
        "terminate",
        Func::from(move |_ctx: Ctx<'_>| {
            let _ = state_term.stdin_tx.send(IpcMessage::Exit { code: 1 });
            Ok::<(), rquickjs::Error>(())
        }),
    )?;

    let loop_ref_ref = loop_ref.clone();
    out.set(
        "ref",
        Func::from(move |_ctx: Ctx<'_>| {
            loop_ref_ref.reference();
            Ok::<(), rquickjs::Error>(())
        }),
    )?;

    let loop_ref_unref = loop_ref.clone();
    out.set(
        "unref",
        Func::from(move |_ctx: Ctx<'_>| {
            loop_ref_unref.unreference();
            Ok::<(), rquickjs::Error>(())
        }),
    )?;

    ctx.spawn_exit_simple(async move {
        let mut stdin = stdin;
        while let Some(msg) = stdin_rx.recv().await {
            let line = match serde_json::to_string(&msg) {
                Ok(l) => format!("{l}\n"),
                Err(_) => continue,
            };
            if stdin.write_all(line.as_bytes()).await.is_err() {
                break;
            }
        }
        Ok(())
    });

    let ctx_for_stdout = ctx.clone();
    let on_event = on_event.clone();
    let child_arc = state.child.clone();
    let loop_ref_stdout = loop_ref.clone();
    ctx.spawn_exit_simple(async move {
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let msg: IpcMessage = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(_) => continue,
            };
            match msg {
                IpcMessage::Online => {
                    let _ = on_event.call::<_, ()>(("online", Undefined));
                },
                IpcMessage::Message { data } => {
                    let value = js_from_json(&ctx_for_stdout, &data)?;
                    let _ = on_event.call::<_, ()>(("message", value));
                },
                IpcMessage::Error { message } => {
                    let err = Object::new(ctx_for_stdout.clone())?;
                    err.set("message", message)?;
                    let _ = on_event.call::<_, ()>(("error", err));
                },
                IpcMessage::Exit { code } => {
                    let _ = on_event.call::<_, ()>(("exit", code));
                    loop_ref_stdout.unreference();
                    break;
                },
            }
        }
        if let Some(mut child) = child_arc.lock().await.take() {
            let _ = child.wait().await;
        }
        Ok(())
    });

    Ok(out)
}

fn worker_parent_post_message(ctx: Ctx<'_>, data: String) -> Result<()> {
    let json: serde_json::Value = serde_json::from_str(&data).or_throw(&ctx)?;
    write_ipc_line(&mut std::io::stdout(), &IpcMessage::Message { data: json }).or_throw(&ctx)
}

/// Run worker script in the current process (worker child).
pub async fn run_worker_entry(ctx: &Ctx<'_>, script: &str, worker_data: &str) -> Result<()> {
    let data_value: Value = if worker_data.is_empty() || worker_data == "null" {
        Undefined.into_value(ctx.clone())
    } else {
        json_parse(ctx, worker_data.as_bytes())?
    };

    ctx.globals().set(
        "__rasterWorkerParentPostMessage",
        Func::from(worker_parent_post_message),
    )?;
    ctx.globals()
        .set("__rasterWorkerBootstrapData", data_value)?;

    let script_path = resolve_worker_filename(ctx, script.to_string())?;
    let import_url = Url::from_file_path(&script_path)
        .map_err(|_| Exception::throw_type(ctx, &format!("Path is not absolute: {script_path}")))?;
    let import_url_lit = serde_json::to_string(import_url.as_str()).or_throw(ctx)?;

    let code = format!(
        r#"
globalThis.workerData = globalThis.__rasterWorkerBootstrapData;
delete globalThis.__rasterWorkerBootstrapData;
await import({import_url_lit});
"#
    );

    let mut opts = rquickjs::context::EvalOptions::default();
    opts.promise = true;
    let promise = ctx.eval_with_options::<Promise, _>(code, opts)?;
    if let Err(err) = promise.into_future::<()>().await.catch(ctx) {
        return Err(err.throw(ctx));
    }

    write_ipc_line(&mut std::io::stdout(), &IpcMessage::Online).or_throw(ctx)?;

    let ctx_read = ctx.clone();
    ctx.clone().spawn_exit_simple(async move {
        let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let msg: IpcMessage = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(_) => continue,
            };
            match msg {
                IpcMessage::Message { data } => {
                    let ctx = ctx_read.clone();
                    if let Ok(v) = js_from_json(&ctx, &data) {
                        let _ = ctx
                            .globals()
                            .get::<_, Function>("__rasterWorkerDispatchParentMessage")
                            .and_then(|f| f.call::<_, ()>((v,)));
                    }
                },
                IpcMessage::Exit { .. } => break,
                _ => {},
            }
        }
        Ok(())
    });

    Ok(())
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    globals.set(
        "__rasterWorkerIsMainThread",
        Func::from(worker_is_main_thread),
    )?;
    globals.set("__rasterWorkerThreadId", Func::from(worker_thread_id))?;
    globals.set("__rasterWorkerSpawn", Func::from(spawn_worker))?;
    Ok(())
}

pub struct WorkerThreadsModule;

impl ModuleDef for WorkerThreadsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |_default| Ok(()))?;
        Ok(())
    }
}

impl From<WorkerThreadsModule> for ModuleInfo<WorkerThreadsModule> {
    fn from(val: WorkerThreadsModule) -> Self {
        ModuleInfo {
            name: "raster_runtime:worker_threads",
            module: val,
        }
    }
}
