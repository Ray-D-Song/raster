---
title: Styling and Layout
description: Use Raster style props with GPUI layout.
---

Raster style props are JSON-like objects, not CSS strings. They are normalized
by the TypeScript runtime and interpreted by the Rust GPUI backend.

```tsx
<View
  style={{
    display: "flex",
    flexDirection: "column",
    gap: 8,
    padding: 12,
    borderWidth: 1,
    borderColor: "#dddddd",
    overflow: "auto",
  }}
/>
```

## Layout

Common fields include:

- `display`, `position`.
- `width`, `height`, `minWidth`, `minHeight`, `maxWidth`, `maxHeight`.
- `flex`, `flexGrow`, `flexShrink`, `flexBasis`, `flexDirection`, `flexWrap`.
- `justifyContent`, `alignItems`, `alignSelf`, `alignContent`.
- `gap`, `rowGap`, `columnGap`.
- `margin`, `padding`, and edge-specific variants.

Numbers are interpreted as GPUI pixels. Percentage-like strings are supported
where the backend has explicit support.

## Visual Style

Supported visual fields include background color, text color, border width,
border color, radius, opacity, and overflow. The Rust backend maps these fields
to GPUI element styling.

## Scrolling

Use `overflow: "auto"` or `overflow: "scroll"` on `View` for ordinary scroll
containers. Use `VirtualList` for large data sets.
