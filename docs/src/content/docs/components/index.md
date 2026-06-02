---
title: Components
description: Current typed components exported by Raster.
---

Import components from `raster/components` or `raster/component`.

```tsx
import { Button, Form, Field, Input, Dialog } from "raster/components";
```

Current typed exports:

- Core: `ConfigProvider`, `View`, `Label`, `Text`, `Input`, `Textarea`.
- Display: `Avatar`, `AvatarGroup`, `Icon`, `Text`.
- Actions: `Button`, `ButtonGroup`, `Checkbox`, `Radio`, `RadioGroup`,
  `Switch`, `Tab`, `TabBar`.
- Forms: `Form`, `Field`, `Input`, `Textarea`, `Select`, `DatePicker`,
  `ColorPicker`.
- Overlays: `Dialog`, `Alert`, `Sheet`, `notification`.
- Data and charts: `VirtualList`, `LineChart`, `BarChart`, `AreaChart`,
  `PieChart`, `CandlestickChart`.

Many `gpui-component` controls are not mapped yet. If a wrapper is not listed
above, it is not a supported public component in this runtime.
