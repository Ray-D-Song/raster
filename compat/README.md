# Compatibility fixtures

Each fixture installs its own locked dependencies. **Vite+** still runs the upstream CLI under Raster and inspects build output without executing it. **Next** uses system Node to produce a standalone deployment, then runs that server under Raster and asserts real HTTP responses.

| Case                               | Versions                   | Flow                                                                                                                                   | Status                                                                                                                                                                                                                                               |
| ---------------------------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Next App Router standalone runtime | Next 16.2.10, React 19.2.5 | Node `next build` (`output: "standalone"`) → Raster runs `.next/standalone/server.js` → HTTP checks on `/`, `/api/health`, `/posts/42`, concurrent `/api/als/:id` | Batch 2 target: inspector probe, timers/promises, AsyncLocalStorage propagation (no `RASTER_RUNTIME_ASYNC_HOOKS`), concurrent ALS isolation. Deferred: worker_threads, Inspector Session/protocol, timer ref/unref, timers/promises setInterval/scheduler. CI uses Node 22.18.0; local runs use system Node. Only the server process is under Raster. |
| Vite+ React library build          | Vite+ 0.2.5, React 19.2.5  | Raster runs `vp build`                                                                                                                 | Observing: local baseline stops while resolving Vite+'s native binding                                                                                                                                                                               |
| better-sqlite3 sync API            | better-sqlite3 11.9.1      | Node `test.cjs` baseline → Raster runs `test.cjs` → stdout contains `better-sqlite3 compat OK`                                         | **Green** with `cargo build --features v8-compat` (Node 24.3.0 / ABI 137 V8 shim). Requires `make compat-better-sqlite3`. |
| v8-hello native addon              | node-addon-api 8.3.1       | `npm run build` (node-gyp) → Node `test.cjs` baseline → Raster runs `test.cjs` → stdout contains `v8-hello compat OK`                    | **Green** with `cargo build --features v8-compat`. Requires `make compat-v8`. |
| N-API hello addon                  | node-addon-api 8.3.1       | `yarn build` (node-gyp) → Node `test.cjs` baseline → Raster (`--features napi`) runs `test.cjs` → stdout contains `napi-hello compat OK` | **Green** when built with `cargo build --features napi` on a dynamically linked target (macOS / glibc). Requires `make compat-napi`. |

Run `make compat-next`, `make compat-vite-plus`, `make compat-better-sqlite3`, `make compat-v8`, or `make compat-napi` after building Raster. V8-native fixtures (`better-sqlite3`, `v8-hello`) require `cargo build --features v8-compat` (the Makefile targets build this automatically). Upgrade a fixture only in a dedicated change that updates its exact dependency versions and lockfile.

**Local `make compat`:** V8 fixtures pin **Node 24.3.0** for ABI 137 (`nvm use 24.3.0`). **Next** and **vite-plus** install steps expect **Node 22.18.0** (CI default) or **≥24.11.0** per upstream engine fields; with only 24.3.0 active, `yarn install` in `compat/vite-plus` may fail on engine checks. **vite-plus** build under Raster still requires `node:readline` (not yet implemented) — see Vite+ row above.

ABI constants for the V8 shim are pinned to **Node 24.3.0 / NODE_MODULE_VERSION 137** (not `refs/node`, which tracks a newer Node). Run `make check-v8-abi` locally after header changes; CI runs the same check on pull requests.

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

Diagnostics land in `compat/better-sqlite3/compat.log` (Node baseline and Raster run stdout/stderr).

`better-sqlite3` uses the V8 C++ native addon ABI (not N-API). Raster provides a QuickJS-backed V8 shim (`v8_compat`, feature `v8-compat`) that implements the Node 24 / ABI 137 callback layouts, templates, accessors, and handle scopes required by the addon.

## v8-hello (V8 C++ addon)

1. **Build addon**: `npm run build` in `compat/v8-hello/` (node-gyp compiles `v8_hello.node`).
2. **Node baseline** (30s): `node test.cjs` — must exit `0` and print `v8-hello compat OK`.
3. **Raster run** (60s): build Raster with `cargo build --features v8-compat`, then run `$RASTER_RUNTIME test.cjs` — same marker.

Diagnostics land in `compat/v8-hello/compat.log`.

## napi-hello (N-API addon)

1. **Build addon**: `yarn build` in `compat/napi-hello/` (node-gyp compiles `hello.node` via N-API).
2. **Node baseline** (30s): `node test.cjs` — must exit `0` and print `napi-hello compat OK`.
3. **Raster run** (60s): build Raster with `cargo build --features napi`, then run `$RASTER_RUNTIME test.cjs` — same marker.

Requires a **dynamically linked** Raster binary (`-rdynamic` / `--export-dynamic`). Static musl container builds (`make raster_runtime-container-*`) do not support `dlopen`.

Diagnostics land in `compat/napi-hello/compat.log`.

### N-API support (Raster shim)

- **`napi_wrap` / `napi_add_finalizer`**: finalizers run when the wrapped object is GC'd, deferred to the next N-API safe point (or env shutdown). Weak references (`refcount == 0`) allow collection; dead weak refs return `undefined` from `napi_get_reference_value`.
- **Handle scopes**: flat value/handle arena with per-scope watermarks; outer-scope `napi_value` handles remain valid across nested scopes. Using handles after their scope closes is undefined (same as Node).
- **Thread-safe functions**: cross-thread `napi_call_threadsafe_function` enqueues work to the per-env driver; callbacks run on the JS thread. `napi_ref_threadsafe_function` / `napi_unref_threadsafe_function` control whether TSFN refs keep the event loop alive.
- **`napi_queue_async_work`**: `execute` runs on a tokio blocking thread; `complete` is posted back to the JS thread via the driver. `execute` must not call N-API except via TSFN.

## Failures and CI

Failures are compatibility results. The workflow is non-blocking (`continue-on-error: true`) until a CI baseline is recorded, then should become a required check.

When a child exits `0` but produces no expected artifact (or HTTP checks fail), `compat/run.mjs` fails with an explicit diagnosis (see `compat/*/compat.log`). On CI failure, `compat.log` and `.next` / `dist` are uploaded as artifacts.
