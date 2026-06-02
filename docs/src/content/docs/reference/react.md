---
title: raster/react
description: React root API.
---

```ts
import { createRoot, createRasterRoot } from "raster/react";
```

## createRoot(options?)

Creates a Raster root backed by a native surface.

```ts
const root = createRoot({
  width: 800,
  height: 600,
  perfdetect: true,
});
```

`createRasterRoot` is an alias of `createRoot`.

## RasterRoot

```ts
interface RasterRoot {
  render(element: ReactElement | null): void;
  clear(): void;
}
```

- `render(element)` synchronously commits React work into the Raster host.
- `clear()` unmounts the current tree and clears the native surface.

## Dev Reload

In `--dev` mode Raster enables an internal root reuse protocol. `createRoot()`
returns the same dev root on reload, `root.clear()` is called before the new app
bundle is evaluated, and hook state resets.
