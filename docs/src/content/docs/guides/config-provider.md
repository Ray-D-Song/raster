---
title: ConfigProvider
description: Configure theme and runtime resources.
---

`ConfigProvider` is a core host component exported from `raster/core` and
`raster/components`.

```tsx
import { ConfigProvider, Button, View } from "raster/components";

<ConfigProvider
  theme={{
    mode: "light",
    radius: 6,
    colors: {
      primary: "#0f7fd1",
      primaryForeground: "#ffffff",
    },
  }}
>
  <View>
    <Button>Save</Button>
  </View>
</ConfigProvider>;
```

## Theme

Supported theme fields include:

- `mode`: `"light"`, `"dark"`, or `"system"`.
- `radius`, `radiusLg`.
- `fontSize`, `fontFamily`, `monoFontSize`, `monoFontFamily`.
- `colors` for core tokens such as `background`, `foreground`, `border`,
  `input`, `primary`, `primaryForeground`, `muted`, `danger`, `success`,
  `warning`, and `info`.

The current implementation applies theme snapshots globally for the surface.
Nested scoped themes are not a separate isolation boundary.

## Text and Resources

`text` and `resources` accept JSON-like objects and are retained with the
provider node. Component-specific use of these fields is intentionally narrow.
