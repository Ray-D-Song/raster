// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

mod ipc;

use std::{
    env,
    io::{self, Write},
    path::PathBuf,
    process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

use raster_runtime_utils::clone_wire::{deserialize, serialize, CloneWirePayload};
use ipc::{
    bootstrap_payload, read_frame_async, split_bootstrap, write_frame_async, write_frame_sync,
    FRAME_BOOTSTRAP, FRAME_ENV_PATCH, FRAME_ERROR, FRAME_EXIT, FRAME_MESSAGE, FRAME_ONLINE,
};
use raster_runtime_context::CtxExtension;
use raster_runtime_utils::{
    event_loop::{self, EventLoopRef},
    module::{export_default, ModuleInfo},
    result::ResultExt,
};
use rquickjs::{
    function::Opt,
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Array, CatchResultExt, Ctx, Exception, Function, IntoJs, Null, Object, Promise, Result,
    Undefined, Value,
};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use url::Url;

pub const ENV_WORKER_INTERNAL: &str = "RASTER_WORKER_INTERNAL";
pub const ENV_WORKER_SCRIPT: &str = "RASTER_WORKER_SCRIPT";

static NEXT_THREAD_ID: AtomicU64 = AtomicU64::new(1);

struct WorkerRegistry {
    channels: Mutex<Vec<Arc<mpsc::UnboundedSender<(u8, CloneWirePayload)>>>>,
}

static WORKER_REGISTRY: OnceLock<WorkerRegistry> = OnceLock::new();

fn registry() -> &'static WorkerRegistry {
    WORKER_REGISTRY.get_or_init(|| WorkerRegistry {
        channels: Mutex::new(Vec::new()),
    })
}

struct WorkerState {
    stdin_tx: Mutex<Option<Arc<mpsc::UnboundedSender<(u8, CloneWirePayload)>>>>,
    loop_ref: Arc<EventLoopRef>,
    child: Arc<tokio::sync::Mutex<Option<tokio::process::Child>>>,
    registry_tx: Arc<mpsc::UnboundedSender<(u8, CloneWirePayload)>>,
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

fn empty_payload() -> CloneWirePayload {
    CloneWirePayload {
        body: Vec::new(),
        transfers: Vec::new(),
    }
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

fn apply_worker_options(
    command: &mut tokio::process::Command,
    ctx: &Ctx<'_>,
    options: &Object<'_>,
) -> Result<()> {
    if let Ok(env_obj) = options.get::<_, Object>("env") {
        for key in env_obj.keys::<String>() {
            let key = key?;
            let val: String = env_obj.get(&key)?;
            command.env(key, val);
        }
    }
    if let Ok(argv) = options.get::<_, Array>("argv") {
        for (i, arg) in argv.iter::<String>().enumerate() {
            command.env(format!("RASTER_WORKER_ARGV_{i}"), arg?);
        }
        command.env("RASTER_WORKER_ARGV_COUNT", argv.len().to_string());
    }
    if let Ok(name) = options.get::<_, String>("name") {
        command.env("RASTER_WORKER_NAME", name);
    }
    let _ = ctx;
    Ok(())
}

fn transfer_options<'js>(transfer: Opt<Value<'js>>) -> Opt<Object<'js>> {
    match transfer.0 {
        None => Opt(None),
        Some(v) if v.is_null() || v.is_undefined() => Opt(None),
        Some(v) => {
            if let Some(obj) = v.as_object() {
                Opt(Some(obj.clone()))
            } else {
                Opt(None)
            }
        }
    }
}

fn emit_worker_exit<'js>(
    ctx: &Ctx<'js>,
    on_event: &Function<'js>,
    code: i32,
    signal: Option<i32>,
) -> Result<()> {
    let exit_obj = Object::new(ctx.clone())?;
    exit_obj.set("code", code)?;
    #[cfg(unix)]
    {
        let signal_val: Value = match signal {
            Some(2) => String::from("SIGINT").into_js(ctx)?,
            Some(9) => String::from("SIGKILL").into_js(ctx)?,
            Some(15) => String::from("SIGTERM").into_js(ctx)?,
            Some(n) => format!("SIG{n}").into_js(ctx)?,
            None => Null.into_value(ctx.clone()),
        };
        exit_obj.set("signal", signal_val)?;
    }
    #[cfg(not(unix))]
    {
        exit_obj.set("signal", Null)?;
    }
    let _ = on_event.call::<_, ()>(("exit", exit_obj));
    Ok(())
}

fn spawn_worker<'js>(
    ctx: Ctx<'js>,
    filename: String,
    options: Object<'js>,
    on_event: Function<'js>,
) -> Result<Object<'js>> {
    let script = resolve_worker_filename(&ctx, filename)?;
    let worker_data: Value = options
        .get("workerData")
        .unwrap_or(Undefined.into_value(ctx.clone()));
    let env_snapshot: Value = match options.get::<_, Value>("rasterEnvironmentSnapshot") {
        Ok(v) => v,
        Err(_) => match options.get::<_, Value>("_environmentSnapshot") {
            Ok(v) => v,
            Err(_) => Object::new(ctx.clone())?.into_value(),
        },
    };

    let worker_wire = serialize(&ctx, worker_data, Opt(None))?;
    let env_wire = serialize(&ctx, env_snapshot, Opt(None))?;
    let bootstrap = bootstrap_payload(worker_wire, env_wire);

    let exe = env::current_exe().or_throw(&ctx)?;
    let thread_id = NEXT_THREAD_ID.fetch_add(1, Ordering::Relaxed);

    let mut command = tokio::process::Command::new(exe);
    command
        .env(ENV_WORKER_INTERNAL, "1")
        .env(ENV_WORKER_SCRIPT, &script)
        .env("RASTER_WORKER_THREAD_ID", thread_id.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true);
    apply_worker_options(&mut command, &ctx, &options)?;
    let mut child = command.spawn().or_throw(&ctx)?;

    let stdin = child.stdin.take().or_throw(&ctx)?;
    let stdout = child.stdout.take().or_throw(&ctx)?;

    let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<(u8, CloneWirePayload)>();
    let stdin_tx = Arc::new(stdin_tx);
    registry()
        .channels
        .lock()
        .unwrap()
        .push(stdin_tx.clone());

    let loop_ref = EventLoopRef::new_referenced(event_loop::rt_key_from_ptr(unsafe {
        rquickjs::qjs::JS_GetRuntime(ctx.as_raw().as_ptr())
    } as *mut std::ffi::c_void));
    let state = Arc::new(WorkerState {
        stdin_tx: Mutex::new(Some(stdin_tx.clone())),
        loop_ref: loop_ref.clone(),
        child: Arc::new(tokio::sync::Mutex::new(Some(child))),
        registry_tx: stdin_tx.clone(),
    });

    let out = Object::new(ctx.clone())?;
    out.set("threadId", thread_id)?;

    let state_post = state.clone();
    out.set(
        "postMessage",
        Func::from(move |ctx: Ctx<'js>, data: Value<'js>, transfer: Opt<Value<'js>>| {
            let wire = serialize(&ctx, data, transfer_options(transfer))?;
            if let Some(tx) = state_post.stdin_tx.lock().unwrap().as_ref() {
                let _ = tx.send((FRAME_MESSAGE, wire));
            }
            Ok::<(), rquickjs::Error>(())
        }),
    )?;

    let state_term = state.clone();
    let on_event_term = on_event.clone();
    out.set(
        "terminate",
        Func::from(move |ctx: Ctx<'js>| {
            if let Some(tx) = state_term.stdin_tx.lock().unwrap().as_ref() {
                let _ = tx.send((FRAME_EXIT, empty_payload()));
            }
            *state_term.stdin_tx.lock().unwrap() = None;
            registry()
                .channels
                .lock()
                .unwrap()
                .retain(|tx| !Arc::ptr_eq(tx, &state_term.registry_tx));
            let child_arc = state_term.child.clone();
            let on_event = on_event_term.clone();
            let ctx_async = ctx.clone();
            ctx.spawn_exit_simple(async move {
                let status = if let Some(mut child) = child_arc.lock().await.take() {
                    let _ = child.kill().await;
                    child.wait().await.ok()
                } else {
                    None
                };
                let code = status.as_ref().and_then(|s| s.code()).unwrap_or(1);
                #[cfg(unix)]
                let signal = status.as_ref().and_then(|s| s.signal());
                #[cfg(not(unix))]
                let signal: Option<i32> = None;
                let _ = emit_worker_exit(&ctx_async, &on_event, code, signal);
                Ok(())
            });
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
        let _ = write_frame_async(&mut stdin, FRAME_BOOTSTRAP, &bootstrap).await;
        while let Some((tag, payload)) = stdin_rx.recv().await {
            if write_frame_async(&mut stdin, tag, &payload).await.is_err() {
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
        let mut reader = BufReader::new(stdout);
        loop {
            let (tag, payload) = match read_frame_async(&mut reader).await {
                Ok(v) => v,
                Err(_) => break,
            };
            match tag {
                FRAME_ONLINE => {
                    let _ = on_event.call::<_, ()>(("online", Undefined));
                },
                FRAME_MESSAGE => {
                    if let Ok(value) = deserialize(&ctx_for_stdout, &payload) {
                        let _ = on_event.call::<_, ()>(("message", value));
                    }
                },
                FRAME_ERROR => {
                    if let Ok(msg) = std::str::from_utf8(&payload.body) {
                        let err = Object::new(ctx_for_stdout.clone())?;
                        err.set("message", msg)?;
                        let _ = on_event.call::<_, ()>(("error", err));
                    }
                },
                FRAME_EXIT => {
                    let code = if payload.body.len() >= 4 {
                        i32::from_be_bytes(payload.body[0..4].try_into().unwrap())
                    } else {
                        0
                    };
                    let _ = emit_worker_exit(&ctx_for_stdout, &on_event, code, None);
                    break;
                },
                _ => {},
            }
        }
        if let Some(mut child) = child_arc.lock().await.take() {
            if let Ok(st) = child.wait().await {
                let code = st.code().unwrap_or(1);
                #[cfg(unix)]
                let signal = st.signal();
                #[cfg(not(unix))]
                let signal: Option<i32> = None;
                let _ = emit_worker_exit(&ctx_for_stdout, &on_event, code, signal);
            }
        }
        loop_ref_stdout.unreference();
        Ok(())
    });

    Ok(out)
}

fn worker_broadcast_env<'js>(ctx: Ctx<'js>, key: Value<'js>, value: Value<'js>) -> Result<()> {
    let key_wire = serialize(&ctx, key, Opt(None))?;
    let value_wire = serialize(&ctx, value, Opt(None))?;
    let mut body = Vec::new();
    body.extend_from_slice(&(key_wire.body.len() as u32).to_be_bytes());
    body.extend_from_slice(&key_wire.body);
    body.extend_from_slice(&(value_wire.body.len() as u32).to_be_bytes());
    body.extend_from_slice(&value_wire.body);
    let payload = CloneWirePayload {
        body,
        transfers: key_wire.transfers,
    };
    for tx in registry().channels.lock().unwrap().iter() {
        let _ = tx.send((FRAME_ENV_PATCH, payload.clone()));
    }
    Ok(())
}

fn worker_parent_post_message<'js>(
    ctx: Ctx<'js>,
    data: Value<'js>,
    transfer: Opt<Value<'js>>,
) -> Result<()> {
    let wire = serialize(&ctx, data, transfer_options(transfer))?;
    write_frame_sync(&mut io::stdout(), FRAME_MESSAGE, &wire).or_throw(&ctx)
}

fn decode_env_patch(payload: &CloneWirePayload) -> std::io::Result<(CloneWirePayload, CloneWirePayload)> {
    let mut pos = 0;
    let key_body = read_env_slice(&payload.body, &mut pos)?;
    let value_body = read_env_slice(&payload.body, &mut pos)?;
    Ok((
        CloneWirePayload {
            body: key_body,
            transfers: Vec::new(),
        },
        CloneWirePayload {
            body: value_body,
            transfers: Vec::new(),
        },
    ))
}

fn read_env_slice(data: &[u8], pos: &mut usize) -> std::io::Result<Vec<u8>> {
    if *pos + 4 > data.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "env patch truncated",
        ));
    }
    let len = u32::from_be_bytes(data[*pos..*pos + 4].try_into().unwrap()) as usize;
    *pos += 4;
    if *pos + len > data.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "env patch truncated",
        ));
    }
    let slice = data[*pos..*pos + len].to_vec();
    *pos += len;
    Ok(slice)
}

/// Run worker script in the current process (worker child).
pub async fn run_worker_entry(ctx: &Ctx<'_>, script: &str) -> Result<()> {
    init(ctx)?;
    ctx.globals().set(
        "__rasterWorkerParentPostMessage",
        Func::from(worker_parent_post_message),
    )?;

    let mut stdin = BufReader::new(tokio::io::stdin());
    let (tag, bootstrap) = read_frame_async(&mut stdin).await.or_throw(ctx)?;
    if tag != FRAME_BOOTSTRAP {
        return Err(Exception::throw_type(ctx, "Worker expected bootstrap frame"));
    }
    let (worker_wire, env_wire) = split_bootstrap(&bootstrap).or_throw(ctx)?;
    let worker_data = deserialize(ctx, &worker_wire)?;
    let env_data = deserialize(ctx, &env_wire)?;
    ctx.globals().set("__rasterWorkerBootstrapData", worker_data)?;
    ctx.globals()
        .set("__rasterWorkerEnvironmentSnapshot", env_data)?;

    let script_path = resolve_worker_filename(ctx, script.to_string())?;
    let import_url = Url::from_file_path(&script_path)
        .map_err(|_| Exception::throw_type(ctx, &format!("Path is not absolute: {script_path}")))?;
    let import_url_lit = serde_json::to_string(import_url.as_str()).or_throw(ctx)?;

    let code = format!(
        r#"
globalThis.workerData = globalThis.__rasterWorkerBootstrapData;
delete globalThis.__rasterWorkerBootstrapData;
(function () {{
  const snap = globalThis.__rasterWorkerEnvironmentSnapshot;
  if (snap && typeof snap === "object") {{
    const m = (globalThis.__rasterWorkerEnvBootstrap = new Map());
    for (const [k, v] of Object.entries(snap)) m.set(k, v);
  }}
  delete globalThis.__rasterWorkerEnvironmentSnapshot;
}})();
await import({import_url_lit});
"#
    );

    let mut opts = rquickjs::context::EvalOptions::default();
    opts.promise = true;
    let promise = ctx.eval_with_options::<Promise, _>(code, opts)?;
    if let Err(err) = promise.into_future::<()>().await.catch(ctx) {
        return Err(err.throw(ctx));
    }

    write_frame_sync(&mut io::stdout(), FRAME_ONLINE, &empty_payload()).or_throw(ctx)?;

    let ctx_read = ctx.clone();
    ctx.clone().spawn_exit_simple(async move {
        let mut reader = BufReader::new(tokio::io::stdin());
        loop {
            let (tag, payload) = match read_frame_async(&mut reader).await {
                Ok(v) => v,
                Err(_) => break,
            };
            match tag {
                FRAME_MESSAGE => {
                    let ctx = ctx_read.clone();
                    if let Ok(v) = deserialize(&ctx, &payload) {
                        let _ = ctx
                            .globals()
                            .get::<_, Function>("__rasterWorkerDispatchParentMessage")
                            .and_then(|f| f.call::<_, ()>((v,)));
                    }
                },
                FRAME_EXIT => break,
                FRAME_ENV_PATCH => {
                    let ctx = ctx_read.clone();
                    if let Ok((key_wire, val_wire)) = decode_env_patch(&payload) {
                        if let (Ok(key), Ok(val)) =
                            (deserialize(&ctx, &key_wire), deserialize(&ctx, &val_wire))
                        {
                            let _ = ctx
                                .globals()
                                .get::<_, Function>("__rasterWorkerApplyEnvPatch")
                                .and_then(|f| f.call::<_, ()>((key, val)));
                        }
                    }
                },
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
    globals.set(
        "__rasterWorkerBroadcastEnv",
        Func::from(worker_broadcast_env),
    )?;
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
