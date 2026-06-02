---
title: Layout Components
description: Layout with View, Form, Field, and tabs.
---

## View

`View` is the core container. It maps to a GPUI `div()` and supports children,
style, scroll overflow, and `onClick`.

```tsx
<View
  style={{
    flexDirection: "column",
    gap: 8,
    padding: 12,
    overflow: "auto",
  }}
>
  {children}
</View>
```

## Form Layout

Use `Form` and `Field` for label/control layouts. `Form` supports `layout`,
`axis`, `size`, `columns`, `labelWidth`, and `labelTextSize`.

## Tab Layout

`TabBar` and `Tab` provide segmented/tab-like native navigation controls. They
do not route application views automatically; update React state in `onClick`
and render the selected content yourself.
