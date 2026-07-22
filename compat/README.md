# Compatibility fixtures

Each fixture installs its own locked dependencies, then asks Raster to run the upstream tool's JavaScript CLI. The build output is inspected but never executed.

| Case | Versions | Raster command | Status |
| --- | --- | --- | --- |
| Next App Router build | Next 16.2.10, React 19.2.5 | `next build` | Observing: past require-hook / `util.promisify` / `dns/promises` / Node semver / `process.on` / `fs.existsSync` / CommonJS relative `import()` (no longer resolves from `eval_script`). Regenerated `compat/next/compat.log` shows Raster exiting `1` with `unhandledRejection Error: IO Error: No such file or directory (os error 2)` from `next/dist/lib/worker.js` while loading `next-build` → `build/index`. SWC/native bindings remain out of scope. |
| Vite+ React library build | Vite+ 0.2.5, React 19.2.5 | `vp build` | Observing: local baseline stops while resolving Vite+'s native binding |

Run `make compat-next` or `make compat-vite-plus` after building Raster. Upgrade a fixture only in a dedicated change that updates its exact dependency versions and lockfile.

Failures are compatibility results. In particular, the Vite+ CLI wrapper and all of its Node API and native-addon requirements run unchanged under Raster. The workflow is non-blocking until a CI baseline is recorded, then should become a required check.

When a Raster child exits `0` but produces no build output directory, `compat/run.mjs` fails with an explicit diagnosis (see `compat/*/compat.log`).
