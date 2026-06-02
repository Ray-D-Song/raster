---
title: Events and State
description: Handle GPUI events from React components.
---

Raster components use controlled React state plus native event callbacks.

```tsx
const [checked, setChecked] = useState(false);

<Switch
  checked={checked}
  onChange={(payload) => setChecked(payload === true || payload === "true")}
/>;
```

## Event Props

Wrapper props whose names start with `on` and whose values are functions are
registered as event handlers.

Events are delivered from GPUI to the JS runtime as runtime commands. Payloads
are JSON-like values.

```tsx
<Button onClick={() => setCount((value) => value + 1)}>
  Count: {count}
</Button>;
```

## Text Inputs

`Input` and `Textarea` expose both structured and simple text callbacks:

- `onChange({ value, eventCount })`
- `onChangeText(value)`
- `onFocus(value)`
- `onBlur(value)`
- `onSubmitEditing(value)`

The `eventCount` field protects controlled text inputs from stale value writes.
