---
title: Action Components
description: Buttons, binary controls, radio controls, and tabs.
---

## Buttons

```tsx
<Button variant="primary" onClick={() => save()}>
  Save
</Button>
```

`Button` supports `label` or children, size, variant, disabled/loading state,
icons, rounded styles, and `onClick`.

## ButtonGroup

`ButtonGroup` consumes direct `Button` children and can be controlled by
`value`.

```tsx
<ButtonGroup value={mode} onChange={setMode}>
  <Button value="read">Read</Button>
  <Button value="write">Write</Button>
</ButtonGroup>
```

## Checkbox, Switch, Radio

`Checkbox`, `Switch`, and `Radio` support controlled checked/selected state and
emit `onChange`/`onClick` payloads.

```tsx
<Switch checked={enabled} onChange={(value) => setEnabled(value === true)} />
```

## Tabs

`TabBar` consumes direct `Tab` children and emits selected indexes as strings.

```tsx
<TabBar selectedIndex={tab} onClick={(value) => setTab(Number(value))}>
  <Tab label="Core" />
  <Tab label="Components" />
</TabBar>
```
