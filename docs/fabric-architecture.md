# Raster Fabric Architecture Contract

Status: Wave 0 contract frozen. Rust agents should import `crate::fabric_types`
for shared ids, handles, payloads, mount items, and error shape instead of
defining local handle schemas.

## End-to-end pipeline

```text
React JS / reconciler
  -> Raster persistent host config
  -> __rasterNative batched binding
  -> Rust ShadowNode / ShadowTree
  -> Rust MountingLayer diff
  -> Rust NativeInstanceTree apply
  -> GPUI owner Entity notify
  -> GPUI render reads retained native instances
```

The production Fabric path does not send full `CommitSnapshot` values from JS.
JS creates opaque native node wrappers and child-set handles, but Rust owns the
canonical shadow tree, revision store, mounting diff, and retained native
instances. GPUI `render()` must not parse JS props, traverse a full JS snapshot,
or call query handlers.

## Field ownership

Rust-generated fields:

- `SurfaceId`: returned by `createSurface`; JS stores and passes it back.
- `NodeTag`: stable host node identity for one surface generation.
- `ShadowRevisionId`: exact immutable shadow record identity; every clone gets a
  new revision id while keeping the same `NodeTag`.
- `NativeNodeHandle`: `{ surface_id, node_tag, revision_id, generation }`.
- `NativeChildSetHandle`: `{ surface_id, child_set_id, generation }`.
- `HandlerSlotId`: runtime-session id returned by native handler registration.
- `MountItem`: emitted only by the Rust mounting layer after a committed child
  set is diffed.
- Surface `generation`: copied into every native handle and incremented when the
  surface/runtime generation is reset.

JS-generated fields:

- `ShadowNodeKind` selection and component `name`.
- React `key`, if present.
- JSON-like `props`, `style`, text content, hidden state, and context payload.
- Event/query binding descriptors: property name, event/query type, and options.
- Handler/query closures in the JS registry. Rust only stores `HandlerSlotId`.

Joint fields:

- `EventBinding` and `QueryBinding` are normalized by JS using native-allocated
  `HandlerSlotId` values. The slot key is scoped by surface, node tag, binding
  kind, and property/event name. Updating a JS closure keeps the same slot id
  for the same live node slot.

## Shared Rust ABI

The minimal Rust ABI is in `src/fabric_types.rs`:

- ID newtypes: `SurfaceId`, `NodeTag`, `ShadowRevisionId`, `HandlerSlotId`.
- Opaque handles: `NativeNodeHandle`, `NativeChildSetHandle`.
- Node schema: `ShadowNodeKind`, `ShadowNodePayload`, `EventBinding`,
  `QueryBinding`.
- Mount schema: `MountItem`.
- Error schema: `FabricError`, `FabricErrorKind`, `FabricResult<T>`.

`ShadowNodePayload` is a normalized full node payload, not a JS source object.
It carries `serde_json::Value` for props/style/context/options so Wave 1 code can
validate and parse at the appropriate layer without adding new dependencies.

## JS opaque wrapper shape

JS host instances must be wrappers around native handles, not source-of-truth
trees:

```ts
type RasterNativeNode = {
  $$typeof: "raster.native-node";
  kind: "host" | "text" | "input" | "textarea" | "widget" | "fragment" | "slot" | "config_provider";
  tag: number; // mirror of Rust NodeTag for debug and slot keys only
  handle: NativeNodeHandle;
  debug?: {
    name?: string;
    key?: string;
    componentStack?: string;
  };
};
```

JS must not maintain a canonical `children` array or serialize a full snapshot.
Child order is expressed by appending node handles to a native child set.

## Binding semantics

Native binding calls are batched by construction:

- `createSurface(options) -> SurfaceId`
- `createNode(surfaceId, kind, name, key, payload) -> NativeNodeHandle`
- `createTextNode(surfaceId, text, payload) -> NativeNodeHandle`
- `cloneNode(handle, payload) -> NativeNodeHandle`
- `createChildSet(surfaceId) -> NativeChildSetHandle`
- `appendChildToSet(childSet, childHandle) -> void`
- `finalizeChildSet(childSet) -> void`
- `commitChildSet(surfaceId, childSet) -> void`
- `deleteNode(handle) -> void` as a GC hint only
- `registerHandlerSlot(surfaceId, nodeTag, kind, property, eventOrQueryType) -> HandlerSlotId`
- `updateHandlerSlot(handlerSlotId, jsFunctionRef) -> void` in the JS registry
- `dropHandlerSlotsForNode(surfaceId, nodeTag) -> void`

Only `commitChildSet` may trigger the mounting layer, native instance updates,
and GPUI owner notify. `createNode`, `createTextNode`, `cloneNode`,
`createChildSet`, `appendChildToSet`, `finalizeChildSet`, handler registration,
and GC hints must not notify GPUI and must not mount anything by themselves.

`finalizeChildSet` makes the child set immutable. `commitChildSet` consumes a
finalized child set and atomically promotes the resulting shadow tree after the
mount diff has been produced. A child set belongs to exactly one surface and one
generation.

## Handle lifecycle

`NodeTag` is stable for a live React host node within one surface generation.
Cloning a node preserves `NodeTag` and allocates a new `ShadowRevisionId`.
Unchanged subtrees may reuse an existing `NativeNodeHandle` in a later child set.

`NativeNodeHandle` identifies an exact immutable shadow revision. Runtime and
shadow-tree code may reject a handle if the generation is stale, the surface does
not match, the revision has been released, or the handle kind does not match the
operation. Unmounted revisions must remain alive until the mount diff consuming
their last committed tree has completed.

`NativeChildSetHandle` is a temporary batch object. It is created during React
render/commit work, receives appended child handles, is finalized, and is
committed at most once. Append after finalize and commit after consume are
errors.

`HandlerSlotId` is stable only for a runtime session and a live node slot. It is
not stable across JS VM reloads, hot reload generation changes, or remounts with
a new `NodeTag`.

## Stale generation rules

Each surface has a current `generation` owned by Rust. Every node handle and
child-set handle is minted with that generation. A handle is stale when:

```text
handle.generation != current_surface.generation
```

Stale handles must be rejected before reading the referenced revision or child
set. The error path should point at the offending handle generation, for example
`$.nativeNodeHandle.generation` or `$.childSet.generation`.

Same-generation old revisions are not stale solely because a newer revision for
the same `NodeTag` exists. They remain valid while still reachable from the
current or work-in-progress trees, subject to later ShadowTree retention rules.

## Error format

Native binding and Fabric layers report errors with:

```json
{
  "kind": "invalid_argument | invalid_handle | stale_handle | missing_field | unsupported | internal",
  "component": "Button",
  "property": "disabled",
  "path": "$.props.disabled",
  "message": "expected boolean, got string"
}
```

`component` and `property` are optional when the error is handle-level rather
than prop-level. `path` is required and uses a JS-object style path rooted at
`$`. The Rust `Display` format is:

```text
{kind} at {path} ({component}.{property}): {message}
```

The parenthesized component/property segment is omitted when those fields are
absent.

## Mount items

`MountItem` is the handoff from ShadowTree diffing to NativeInstanceTree. It is
Rust-generated and never sent by JS. The initial frozen variants are:

- `CreateInstance { tag, revision_id, payload }`
- `UpdateInstance { tag, revision_id, payload }`
- `UpdateEventBindings { tag, revision_id, events, queries }`
- `InsertChild { parent, child, index }`
- `RemoveChild { parent, child }`
- `MoveChild { parent, child, from, to }`
- `DeleteInstance { tag }`
- `UpdatePortal { tag, revision_id, portal_state }`
- `UpdateContext { tag, revision_id, context }`

`parent: None` means the surface root container. Replace semantics can be
expressed as delete/create plus child operations unless a later wave adds a
specialized replace item.

## Legacy deletion gate

The final Fabric migration is not complete until all of these are true:

- Production rendering no longer calls `__rasterCommitRender`.
- `CommitSnapshot`, `parse_commit_snapshot`, and `render_snapshot` are absent
  from the production render path.
- `packages/raster/src` has no production `serializeNode` or full snapshot
  `commitRoot` path.
- GPUI render reads retained native instances only; render-time parser/query
  diagnostics stay at zero.
- Existing behavior tests and new Fabric shadow tree, native binding, mounting,
  and native instance tests pass.
