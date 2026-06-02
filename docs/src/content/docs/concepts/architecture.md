---
title: Architecture
description: The current JavaScript-to-GPUI pipeline.
---

Raster is split into a TypeScript runtime, a JavaScript VM, a Rust native host
binding, a retained GPUI tree, and GPUI owner views.

```text
React components
  -> raster/react mutation host config
  -> __rasterNative calls
  -> NativeHostState
  -> MountMutationBatch queue
  -> RetainedTree
  -> SurfaceOwnerView / NodeOwnerView
  -> GPUI elements and gpui-component widgets
```

## TypeScript Runtime

`packages/raster` owns the runtime-facing TypeScript packages:

- `raster/react` creates Raster roots and drives React reconciliation.
- `raster/core` defines intrinsic host elements and normalized native payloads.
- `raster/component` and `raster/components` define typed component wrappers.

The generated runtime bundle is embedded into Rust from
`src/runtime/js/generated/runtime_bundle.js`.

## JavaScript VM

`src/js_runtime` starts the vendored `raster_runtime` VM, installs Raster host
modules, evaluates the generated runtime bundle, and evaluates the app bundle.
Runtime commands from GPUI events invoke registered JS handlers.

In `--dev` mode the VM installs a `node:fs.watch` watcher for the app bundle.
When Vite rewrites the bundle, Raster clears the current root with `root.clear()`
and evaluates the new app module using a unique reload name.

## Native Binding

The renderer uses mutation mode. It calls methods such as `createSurface`,
`createNode`, `appendChild`, `removeChild`, `updateNode`, `deleteNode`, and
`clearSurface` on `globalThis.__rasterNative`.

`NativeHostState` collects one React commit into a `MountMutationBatch`. The
batch is sent to the GPUI app thread and applied to `RetainedTree`.

## GPUI Backend

`RasterRootView` owns the retained tree, owner registries, notification center,
theme snapshot, and window-level sheet/dialog/alert state.

Regular layout flows through `SurfaceOwnerView` and `NodeOwnerView`. Components
with native transient state, such as inputs, selects, date pickers, color
pickers, virtual lists, and charts, keep owner state in their node owner.

GPUI render should read retained Rust state. It should not call JavaScript or
reparse source JSX.
