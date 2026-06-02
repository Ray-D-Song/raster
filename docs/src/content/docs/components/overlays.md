---
title: Overlay Components
description: Dialog, Alert, Sheet, and notifications.
---

Overlay components are controlled by React state. Rust synchronizes the first
open node of each overlay type for the current surface.

## Dialog

```tsx
<Dialog
  open={dialogOpen}
  title="Confirm"
  confirm
  okText="OK"
  cancelText="Cancel"
  onOk={() => setDialogOpen(false)}
  onOpenChange={(payload) => setDialogOpen(payload.open)}
>
  <Text>Continue?</Text>
</Dialog>
```

`Dialog` supports `children`, `title`, `confirm`, `okText`, `cancelText`,
width options, overlay behavior, keyboard closing, close button, `onOk`,
`onCancel`, and `onOpenChange`.

## Alert

`Alert` maps to native `AlertDialog`.

```tsx
<Alert
  open={alertOpen}
  title="Delete account?"
  description="This action cannot be undone."
  showCancel
  okText="Continue"
  onOpenChange={(payload) => setAlertOpen(payload.open)}
/>
```

## Sheet

```tsx
<Sheet
  open={sheetOpen}
  title="Details"
  placement="right"
  onOpenChange={(payload) => setSheetOpen(payload.open)}
>
  <Text>Sheet content</Text>
</Sheet>
```

`SheetOpenChangePayload.reason` is `"close-button"`, `"escape"`, `"overlay"`,
or `"controlled"`.

## Notifications

Notifications are command-driven and do not enter the retained tree.

```tsx
notification.show({ id: "saved", type: "success", message: "Saved" });
notification.dismiss("saved");
notification.clear();
```
