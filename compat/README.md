# Compatibility fixtures

Each fixture installs its own locked dependencies. **Vite+** still runs the upstream CLI under Raster and inspects build output without executing it. **Next** uses system Node to produce a standalone deployment, then runs that server under Raster and asserts real HTTP responses.

| Case                               | Versions                   | Flow                                                                                                                                   | Status                                                                                                                                                                                                                                               |
| ---------------------------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Next App Router standalone runtime | Next 16.2.10, React 19.2.5 | Node `next build` (`output: "standalone"`) → Raster runs `.next/standalone/server.js` → HTTP checks on `/`, `/api/health`, `/posts/42` | Observing: Node standalone build succeeds; Raster loads past `process.chdir`, `process.pid`, `require("v8")`, and `require("constants")`. Current first error: `Cannot find module 'node:inspector'` (Next `console-dim.external.js` via `node-environment`). Isolated repro prints a clear stack; full `server.js` may exit 1 with empty stderr after Next patches `console`. CI uses Node 22.18.0; local runs use the currently active system Node (`process.execPath`). Only the server process is under Raster. |
| Vite+ React library build          | Vite+ 0.2.5, React 19.2.5  | Raster runs `vp build`                                                                                                                 | Observing: local baseline stops while resolving Vite+'s native binding                                                                                                                                                                               |

Run `make compat-next` or `make compat-vite-plus` after building Raster. Upgrade a fixture only in a dedicated change that updates its exact dependency versions and lockfile.

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
7. Always stop the server (SIGTERM, then SIGKILL after 5s).

Diagnostics land in `compat/next/compat.log` (Node build command/output, Raster start command/output, readiness last error, each HTTP check). Static assets (`.next/static`, CSS, images) are **not** copied or verified in this fixture; coverage is HTML SSR, API, and dynamic route only.

A green Next result means **Node-built standalone + Raster runtime HTTP**, not “Raster can execute `next build`”.

## Failures and CI

Failures are compatibility results. The workflow is non-blocking (`continue-on-error: true`) until a CI baseline is recorded, then should become a required check.

When a child exits `0` but produces no expected artifact (or HTTP checks fail), `compat/run.mjs` fails with an explicit diagnosis (see `compat/*/compat.log`). On CI failure, `compat.log` and `.next` / `dist` are uploaded as artifacts.
