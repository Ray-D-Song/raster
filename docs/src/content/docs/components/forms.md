---
title: Form Components
description: Inputs, fields, validation, select, date, and color controls.
---

## Text Controls

Use `Input` and `Textarea` for controlled text editing.

```tsx
<Input value={name} onChangeText={setName} placeholder="Name" />
<Textarea value={bio} onChangeText={setBio} rows={4} />
```

## Form and Field

`Form` provides native form layout. `Field` wraps controls with label,
description, required marker, visibility, alignment, and grid positioning.

```tsx
<Form columns={2} labelWidth={120}>
  <Field
    label="Name"
    value={name}
    description="Shown when the field is valid."
    validate={(value) => ({
      error: String(value ?? "").length < 3,
      message: "Name must be at least 3 characters.",
    })}
  >
    <Input value={name} onChangeText={setName} />
  </Field>
</Form>
```

`Field.validate` is a JavaScript wrapper feature. It defaults to a 300ms
debounce. When it returns `{ error: true, message }`, Raster passes the message
to native `Field.description` and renders it in the error color.

## Select

`Select` uses JSON data, not JSX option children.

```tsx
<Select
  value={channel}
  options={[
    { id: "stable", label: "Stable", value: "stable" },
    { id: "nightly", label: "Nightly", value: "nightly" },
  ]}
  onChange={(payload) => setChannel(String(payload.value ?? ""))}
/>
```

## DatePicker and ColorPicker

`DatePicker` accepts ISO date strings. `ColorPicker` accepts CSS color or hex
strings and emits native hex values.

```tsx
<DatePicker value={date} cleanable onChange={(payload) => setDate(payload.value as string | null)} />
<ColorPicker value={color} icon="palette" onChange={(payload) => setColor(payload.value)} />
```
