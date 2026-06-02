---
title: Runtime Contracts
description: Boundaries between React, JavaScript, Rust, and GPUI.
---

Raster keeps a narrow contract between JavaScript and Rust.

## JavaScript Owns

- React component state and hook state.
- Event and query closures.
- Wrapper behavior implemented in TypeScript, such as `Field.validate`.
- App bundle evaluation and dev reload entry execution.

## Rust Owns

- Native ids, node handles, surfaces, handler slots, and mutation queues.
- Retained tree state used by GPUI render.
- Native component owner state and window-level overlays.
- Event dispatch from GPUI back into JavaScript runtime commands.

## Props

Props sent through the retained tree must be JSON-like unless the API explicitly
marks them as event or query functions. Component wrappers split:

- `onX` function props into event handlers.
- `getX` function props into query handlers.
- all remaining props into serializable component props.

## Events

Native events are asynchronous Rust-to-JS runtime commands. Handler payloads are
serialized as `NodeValue` and delivered to the registered closure. The React
renderer flushes synchronous work after handler invocation.

## Queries

Queries are allowed only for interaction-time callbacks where an API explicitly
supports them. GPUI render must not call query handlers.

## Root Lifecycle

`createRoot(options)` creates a Raster surface. `root.render(element)` commits a
React tree. `root.clear()` unmounts the current tree and submits an empty surface
to Rust. Dev reload reuses one root and calls `clear()` before evaluating the
new app bundle.
