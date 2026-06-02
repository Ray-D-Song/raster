---
title: React Renderer
description: Use Raster's React host renderer.
---

Use `createRoot` from `raster/react` to create a native Raster surface.

```tsx
import { createRoot } from "raster/react";

const root = createRoot({ width: 800, height: 600, perfdetect: true });
root.render(<App />);
```

## Root Options

| Option | Type | Description |
| --- | --- | --- |
| `width` | `number | null` | Initial surface width used by the native window. |
| `height` | `number | null` | Initial surface height used by the native window. |
| `perfdetect` | `boolean | null` | Enables the GPUI performance overlay. |

## Clearing a Root

`root.clear()` unmounts the current React tree and submits an empty surface to
Rust.

```tsx
const root = createRoot();
root.render(<App />);
root.clear();
```

`clear()` is also used by dev bundle hot reload. It is a public root API, not a
shortcut for users to call `render(null)`.

## Host Elements

Raster does not expose DOM nodes. JSX host elements are Raster native elements:
`View`, `Label`, `Input`, `Textarea`, `Widget`, `Slot`, and `ConfigProvider`.

Prefer typed components from `raster/components` over raw `Widget` unless you
are adding a new native component mapping.
