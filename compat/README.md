# Compatibility fixtures

Each fixture installs its own locked dependencies. **Vite+** still runs the upstream CLI under Raster and inspects build output without executing it. **Next** uses system Node to produce a standalone deployment, then runs that server under Raster and asserts real HTTP responses.

| Case                               | Versions                   | Flow                                                                                                                                   | Status                                                                                                                                                                                                                                               |
| ---------------------------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Next App Router standalone runtime | Next 16.2.10, React 19.2.5 | Node `next build` (`output: "standalone"`) → Raster runs `.next/standalone/server.js` → HTTP checks on `/`, `/api/health`, `/posts/42`, concurrent `/api/als/:id` | Batch 2 target: inspector probe, timers/promises, AsyncLocalStorage propagation (no `RASTER_RUNTIME_ASYNC_HOOKS`), concurrent ALS isolation. Deferred: worker_threads, Inspector Session/protocol, timer ref/unref, timers/promises setInterval/scheduler. CI uses Node 22.18.0; local runs use system Node. Only the server process is under Raster. |
| Vite+ React library build          | Vite+ 0.2.5, React 19.2.5  | Raster runs `vp build`                                                                                                                 | Observing: local baseline stops while resolving Vite+'s native binding                                                                                                                                                                               |
| better-sqlite3 sync API            | better-sqlite3 11.9.1      | Node `test.cjs` baseline → Raster runs `test.cjs` → stdout contains `better-sqlite3 compat OK`                                         | **Red**: uses V8 C++ ABI (not N-API). Node baseline passes; Raster run remains blocked until a V8 shim or Rust-native alias exists. |
| N-API hello addon                  | node-addon-api 8.3.1       | `yarn build` (node-gyp) → Node `test.cjs` baseline → Raster (`--features napi`) runs `test.cjs` → stdout contains `napi-hello compat OK` | **Green** when built with `cargo build --features napi` on a dynamically linked target (macOS / glibc). Requires `make compat-napi`. |

Run `make compat-next`, `make compat-vite-plus`, `make compat-better-sqlite3`, or `make compat-napi` after building Raster. Upgrade a fixture only in a dedicated change that updates its exact dependency versions and lockfile.

## Next (standalone runtime)

1. Delete any previous `compat/next/.next`.
2. Build with **system Node** (`process.execPath`), not Raster: `node node_modules/next/dist/bin/next build` (120s wall-clock timeout).
3. Require `.next/standalone/server.js`.
4. Start that entry with **Raster** (`HOSTNAME=127.0.0.1`, dynamic `PORT`, `NODE_ENV=production`, `NEXT_TELEMETRY_DISABLED=1`, cwd = `.next/standalone`).
5. Poll `GET /api/health` for up to 30s (do not rely on console "Ready" text).
6. Assert (each request aborts after 5s):
   - `GET /` → 200, body contains `Raster Next compatibility fixture`
   - `GET /api/health` → 200, JSON `{ "status": "ok" }`
   - `GET /posts/42` → 200, body contains `Post 42`
   - Concurrent `GET /api/als/{id}` for multiple ids → each JSON `{ "id": "<same id>" }` (AsyncLocalStorage isolation across await + timers)
7. Always stop the server (SIGTERM, then SIGKILL after 5s). Raster is started without `RASTER_RUNTIME_ASYNC_HOOKS`.

Diagnostics land in `compat/next/compat.log` (Node build command/output, Raster start command/output, readiness last error, each HTTP check). Static assets (`.next/static`, CSS, images) are **not** copied or verified in this fixture; coverage is HTML SSR, API, dynamic route, and concurrent ALS isolation only.

A green Next result means **Node-built standalone + Raster runtime HTTP**, not “Raster can execute `next build`”.

## better-sqlite3 (sync API)

1. **Node baseline** (30s timeout): `node test.cjs` in `compat/better-sqlite3/` — must exit `0` and print `better-sqlite3 compat OK`. Validates the fixture, dependency install, and test script. If this fails, Raster is not started.
2. **Raster run** (60s timeout): `$RASTER_RUNTIME test.cjs` with the same cwd — same exit code and stdout marker. This is the compatibility acceptance step.

`test.cjs` exercises (sync API only):

- In-memory `Database`, `exec`, `prepare`, `run`, `get`, `all`
- `transaction()` commit and rollback on thrown error
- File-backed database, `pragma('journal_mode = WAL')`, cleanup

Deferred: `worker_threads`, `backup()`, custom SQL functions, `loadExtension()`, ESM import.

Diagnostics land in `compat/better-sqlite3/compat.log` (Node baseline and Raster run stdout/stderr). `better-sqlite3` uses the V8 native addon ABI, not N-API; expect the Raster step to fail even when N-API addon loading is enabled.

## napi-hello (N-API addon)

1. **Build addon**: `yarn build` in `compat/napi-hello/` (node-gyp compiles `hello.node` via N-API).
2. **Node baseline** (30s): `node test.cjs` — must exit `0` and print `napi-hello compat OK`.
3. **Raster run** (60s): build Raster with `cargo build --features napi`, then run `$RASTER_RUNTIME test.cjs` — same marker.

Requires a **dynamically linked** Raster binary (`-rdynamic` / `--export-dynamic`). Static musl container builds (`make raster_runtime-container-*`) do not support `dlopen`.

Diagnostics land in `compat/napi-hello/compat.log`.

### N-API limitations (Raster shim)

- **`napi_wrap` / `napi_add_finalizer`**: finalizers run at env shutdown (`prepare_shutdown`), not when the JS object is GC'd. `napi_create_external` uses a QuickJS class finalizer and does run on GC.
- **Nested handle scopes**: `napi_value` indices are per innermost escapable or handle scope. Opening a nested ordinary handle scope while holding handles from an outer scope can make those handles unresolvable. Prefer a single scope per callback, or re-fetch values inside the inner scope.
- **Thread-safe functions**: TSFN callbacks are drained on the main JS thread only. `napi_call_threadsafe_function` from a worker thread is undefined behavior (QuickJS is not thread-safe). `napi_ref_threadsafe_function` / `napi_unref_threadsafe_function` are no-ops.
- **`napi_queue_async_work`**: `execute` runs on a short-lived worker `std::thread` (the JS thread blocks on `join()` until it returns); `complete` runs on the JS thread afterward. Addons must not call N-API from `execute` except via TSFN.

## Failures and CI

Failures are compatibility results. The workflow is non-blocking (`continue-on-error: true`) until a CI baseline is recorded, then should become a required check.

When a child exits `0` but produces no expected artifact (or HTTP checks fail), `compat/run.mjs` fails with an explicit diagnosis (see `compat/*/compat.log`). On CI failure, `compat.log` and `.next` / `dist` are uploaded as artifacts.
