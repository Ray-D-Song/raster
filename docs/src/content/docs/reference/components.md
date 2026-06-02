---
title: raster/components
description: Typed native component wrappers.
---

`raster/component` and `raster/components` expose the same component wrappers.

## Current Component Names

```ts
type ComponentName =
  | "Avatar"
  | "AvatarGroup"
  | "Alert"
  | "Button"
  | "ButtonGroup"
  | "Checkbox"
  | "ColorPicker"
  | "DatePicker"
  | "Dialog"
  | "Field"
  | "Form"
  | "Icon"
  | "LineChart"
  | "BarChart"
  | "AreaChart"
  | "PieChart"
  | "CandlestickChart"
  | "Radio"
  | "RadioGroup"
  | "Select"
  | "Sheet"
  | "Switch"
  | "Tab"
  | "TabBar"
  | "VirtualList";
```

## Wrapper Behavior

Typed wrappers split props into native channels:

- function props named `onX` become events.
- function props named `getX` become queries.
- other props must be JSON-like and are sent as component props.

## Command APIs

`notification` is command-based:

```ts
notification.show({ type: "success", message: "Saved" });
notification.dismiss("id");
notification.clear();
```

Chart refs expose:

```ts
interface ChartRef {
  appendData(rowOrRows: ChartDatum | ChartDatum[]): void;
  replaceData(rows: ChartDatum[]): void;
  clearData(): void;
}
```
