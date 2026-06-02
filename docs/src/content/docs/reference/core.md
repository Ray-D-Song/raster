---
title: raster/core
description: Core host elements and shared types.
---

`raster/core` exports intrinsic host wrappers and shared runtime types.

## Host Components

- `View`: container, GPUI `div()` projection, style, children, scroll overflow,
  and `onClick`.
- `Label`: native text label.
- `Text`: alias-style text wrapper that maps through `Label`.
- `Input`: single-line text control.
- `Textarea`: multiline text control.
- `Slot`: protocol node for components that explicitly consume slots.
- `ConfigProvider`: theme, text, and resource provider.
- `Widget`: low-level escape hatch used by typed components.

## Shared Types

Core exports JSON types, event/query handler types, root options, native handles,
style types, theme types, and component base prop types.

```ts
type RasterEventHandler<T = unknown> = (payload: T) => void;
type RasterQueryHandler<TPayload = unknown, TResult = unknown> =
  (payload: TPayload) => TResult;
```

Prefer typed wrappers from `raster/components`. Use `Widget` only when adding or
experimenting with a new native component mapping.
