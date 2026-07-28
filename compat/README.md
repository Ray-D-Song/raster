# Compatibility fixtures

Each fixture installs its own locked dependencies. **Vite+** still runs the upstream CLI under Raster and inspects build output without executing it. **Next** uses system Node to produce a standalone deployment, then runs that server under Raster and asserts real HTTP responses.

| Case                               | Versions                   | Flow                                                                                                                                                              | Status                                                                                                                                                                                                                                                                                                                                                |
| ---------------------------------- | -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Next App Router standalone runtime | Next 16.2.10, React 19.2.5 | Node `next build` (`output: "standalone"`) → Raster runs `.next/standalone/server.js` → HTTP checks on `/`, `/api/health`, `/posts/42`, concurrent `/api/als/:id` | Batch 2 target: inspector probe, timers/promises, AsyncLocalStorage propagation (no `RASTER_RUNTIME_ASYNC_HOOKS`), concurrent ALS isolation. Deferred: worker_threads, Inspector Session/protocol, timer ref/unref, timers/promises setInterval/scheduler. CI uses Node 22.18.0; local runs use system Node. Only the server process is under Raster. |
| Vite+ React library build          | Vite+ 0.2.5, React 19.2.5  | Raster runs `vp build`                                                                                                                                            | Observing: local baseline stops while resolving Vite+'s native binding                                                                                                                                                                                                                                                                                |
| better-sqlite3 sync API            | better-sqlite3 11.9.1      | Node `test.cjs` baseline → Raster runs `test.cjs` → stdout contains `better-sqlite3 compat OK`                                                                    | **Green** with `cargo build --features v8-compat` (Node 24.3.0 / ABI 137 V8 shim). Requires `make compat-better-sqlite3`.                                                                                                                                                                                                                             |
| v8-hello native addon              | node-addon-api 8.3.1       | `npm run build` (node-gyp) → Node `test.cjs` baseline → Raster runs `test.cjs` → stdout contains `v8-hello compat OK`                                             | **Green** with `cargo build --features v8-compat`. Requires `make compat-v8`.                                                                                                                                                                                                                                                                         |
| N-API hello addon                  | node-addon-api 8.3.1       | `yarn build` (node-gyp) → Node `test.cjs` baseline → Raster (`--features napi`) runs `test.cjs` → stdout contains `napi-hello compat OK`                          | **Green** when built with `cargo build --features napi` on a dynamically linked target (macOS / glibc). Requires `make compat-napi`.                                                                                                                                                                                                                  |
| mysql2 async API                   | mysql2 3.23.2, MySQL 8.4   | Docker `mysql:8.4` temp container → Node `test.cjs` baseline → Raster runs `test.cjs` → stdout contains `mysql2 compat OK`                                        | **Observing**: CI job `mysql-compat` is non-blocking (`continue-on-error: true`). Requires Docker and `make compat-mysql`.                                                                                                                                                                                                                            |

Run `make compat-next`, `make compat-vite-plus`, `make compat-better-sqlite3`, `make compat-mysql`, `make compat-v8`, or `make compat-napi` after building Raster. V8-native fixtures (`better-sqlite3`, `v8-hello`) require `cargo build --features v8-compat` (the Makefile targets build this automatically). Upgrade a fixture only in a dedicated change that updates its exact dependency versions and lockfile.

**Local `make compat`:** V8 fixtures pin **Node 24.3.0** for ABI 137 (`nvm use 24.3.0`). **Next** and **vite-plus** install steps expect **Node 22.18.0** (CI default) or **≥24.11.0** per upstream engine fields; with only 24.3.0 active, `yarn install` in `compat/vite-plus` may fail on engine checks. **vite-plus** build under Raster still requires `node:readline` (not yet implemented) — see Vite+ row above. **`make compat` includes `compat-mysql`**, which requires a running Docker daemon and will fail on Raster until mysql2 compatibility gaps are closed. The **mysql2** fixture baseline uses **Node 22.18.0** (fixture `engines` accept **≥22.18.0**).

**V8 C++ shim platform support:** Linux (x64, arm64) and macOS (Apple Silicon and Intel) are tested in CI. **Windows is not supported** for `v8-compat` — the C++ shim is Unix-only (`build.rs` links a static archive; no `rstr.dll` proxy path). Use WSL or a Linux/macOS host for V8-native addons (`better-sqlite3`, `v8-hello`).

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

`test.cjs` exercises:

- `node::Buffer::Copy` / `Data` / `Length`
- External `Buffer::New` with finalizer callback
- `Persistent::SetWeak` + `ClearWeak` (weak callback must not run after clear)

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

## mysql2 (async API)

`make compat-mysql` manages the full test lifecycle:

1. Starts a temporary **mysql:8.4** Docker container with a random local port (no fixed container name).
2. Waits for Docker health checks to report `healthy` (up to 120s).
3. **Node baseline** (30s timeout): `node test.cjs` in `compat/mysql2/` — must exit `0` and print `mysql2 compat OK`. If this fails, Raster is not started.
4. **Raster run** (60s timeout): `$RASTER_RUNTIME test.cjs` with the same cwd — same exit code and stdout marker.
5. Stops and removes the container on success, failure, or interrupt (`SIGINT` / `SIGTERM`).

**Requirements:** Docker daemon available locally or on the CI runner. You do **not** need to pre-install, pre-configure, or manually start MySQL. Raster is built **without** `napi` or `v8-compat`. Node **22.18.0** (fixture `engines` accept **≥22.18.0**), mysql2 **3.23.2**, MySQL **8.4 LTS** with default `caching_sha2_password` authentication.

**Environment variables** when running `test.cjs` directly against an external database (defaults shown):

| Variable         | Default         |
| ---------------- | --------------- |
| `MYSQL_HOST`     | `127.0.0.1`     |
| `MYSQL_PORT`     | `3306`          |
| `MYSQL_DATABASE` | `raster_compat` |
| `MYSQL_USER`     | `raster`        |
| `MYSQL_PASSWORD` | `raster`        |

`make compat-mysql` overrides these with values from its temporary container.

`test.cjs` exercises:

- Module entry (`mysql2` and `mysql2/promise`)
- Promise `createConnection`, `execute`, transactions (commit/rollback), and server error propagation
- Callback `createConnection`, `connect`, `query`, and `end`
- `createPool`, concurrent `getConnection`, parameterized `pool.execute`, `pool.query`, and `pool.end`

The fixture keeps mysql2's default `enableKeepAlive: true` and default authentication flow. It does **not** disable keepalive, switch to legacy auth plugins, or use MariaDB to bypass compatibility gaps.

**Current Raster gaps (expected):** `Socket.setNoDelay()` is the first known failure. Further gaps may include `Socket.setKeepAlive()` and RSA `publicEncrypt` required for non-TLS `caching_sha2_password` on MySQL 8.4.

**CI:** The `mysql-compat` job calls `make compat-mysql` on Ubuntu (Docker is available on the runner). It uses `continue-on-error: true` while Raster gaps are being closed — failures are recorded and `compat/mysql2/compat.log` is uploaded, but the job does not block merges. Remove `continue-on-error` only when Raster reliably prints `mysql2 compat OK`.

Diagnostics land in `compat/mysql2/compat.log` (Docker image/container/port, health transitions, container logs on failure, database host/port/database/user, Node baseline and Raster run stdout/stderr, container stop result). Passwords are never logged.

## Failures and CI

Most compatibility failures block merges. The workflow runs `better-sqlite3` and `v8-hello` on Ubuntu and macOS with Node 24.3.0. The **`mysql-compat` job is an exception**: it is non-blocking (`continue-on-error: true`) until Raster passes the full fixture.

When a child exits `0` but produces no expected artifact (or HTTP checks fail), `compat/run.mjs` fails with an explicit diagnosis (see `compat/*/compat.log`). On CI failure, `compat.log` and `.next` / `dist` are uploaded as artifacts.
