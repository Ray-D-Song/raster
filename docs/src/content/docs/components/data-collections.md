---
title: Data and Collections
description: Select, virtual list, and chart data.
---

## Select

`Select` consumes JSON options or sections.

```tsx
<Select
  value="stable"
  options={[
    { id: "stable", label: "Stable", value: "stable" },
    { id: "nightly", label: "Nightly", value: "nightly" },
  ]}
/>
```

## VirtualList

`VirtualList` uses `items`, `itemSize`, and a JavaScript `renderItem` wrapper.
The wrapper generates retained row children; Rust renders only the visible
range.

```tsx
<VirtualList
  items={rows}
  itemSize={32}
  renderItem={({ item }) => <Text>{item.label}</Text>}
  onVisibleRangeChange={({ start, end }) => console.log(start, end)}
/>
```

Do not use render-time queries for row data.

## Charts

Raster exports `LineChart`, `BarChart`, `AreaChart`, `PieChart`, and
`CandlestickChart`.

```tsx
const lineRef = useRef<ChartRef | null>(null);

<LineChart
  ref={lineRef}
  data={data}
  maxDataLength={100}
  x="month"
  y="value"
  stroke="#0f7fd1"
  style={{ height: 240 }}
/>;

lineRef.current?.appendData({ month: "May", value: 72 });
```

The `data` prop is controlled replacement data. Ref commands mutate chart owner
state without writing back into React state:

- `appendData(rowOrRows)`
- `replaceData(rows)`
- `clearData()`
