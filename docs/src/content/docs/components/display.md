---
title: Display Components
description: Text, avatar, icon, and chart display components.
---

## Text and Labels

`Text` maps to the same native label path as `Label`.

```tsx
<Text style={{ color: "#0f7fd1" }}>Hello GPUI</Text>
```

`Label` supports secondary text, masking, highlighting, and selection.

## Avatar and AvatarGroup

```tsx
<AvatarGroup limit={2} ellipsis>
  <Avatar name="Raster" />
  <Avatar name="GPUI" />
  <Avatar name="JS" placeholder="user" />
</AvatarGroup>
```

## Icon

`Icon` accepts `name` or `icon`, size, color, rotation, and custom path data.

```tsx
<Icon name="settings" color="#0f7fd1" />
```

## Charts

Chart components are display components with owner state for data. See
[Data and Collections](/components/data-collections/) for chart data updates.
