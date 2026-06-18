import type {
  JsonObject,
  JsonValue,
  RasterHandlerSlotId,
  RasterHandlerSlotKind,
  RasterNativeBinding,
  RasterNativeChildSet,
  RasterNativeEventBinding,
  RasterNativeJsFunctionRef,
  RasterNativeMaterializeChildUpdate,
  RasterNativeMaterializeResult,
  RasterNativeMaterializeSpec,
  RasterNativeNode,
  RasterNativeNodeHandle,
  RasterNativeNodeKind,
  RasterNativeQueryBinding,
  RasterNodeTag,
  RasterRootOptions,
  RasterShadowNodePayload,
  RasterSurfaceId,
} from "./types.js";

type HandlerId = RasterHandlerSlotId;
type RasterHandler = (payload: unknown) => unknown;
type HostProps = Record<string, unknown>;
type HandlerKind = RasterHandlerSlotKind;
type HandlerSlotKey = string;

interface HandlerRecord {
  handler: RasterHandler;
  eventType: string | null;
  surfaceId: RasterSurfaceId;
  nodeTag: RasterNodeTag;
  name: string;
}

interface RasterHandlerRegistry {
  handlers: Map<HandlerId, HandlerRecord>;
  handlerSlots: Map<HandlerSlotKey, HandlerId>;
}

interface RasterEventMetadata {
  type: string | null;
  timeStamp: number;
}

export interface RasterFabricContainer {
  $$typeof: "raster.native-container";
  surfaceId: RasterSurfaceId;
  parent: null;
}

type RasterRuntimeGlobal = typeof globalThis & {
  __rasterNative?: RasterNativeBinding;
  __rasterFlushSyncWork?: () => void;
  __rasterInvokeHandler?: (id: HandlerId, payload: unknown) => unknown;
  __rasterInvokeHandlerJson?: (id: HandlerId, payloadJson: string) => unknown;
  __rasterInvokeHandlersJson?: (callsJson: string) => unknown[];
  __rasterInvokeQuery?: (id: HandlerId, payload: unknown) => unknown;
  __rasterInvokeQueryJson?: (id: HandlerId, payloadJson: string) => string;
  __rasterHandlerRegistry?: RasterHandlerRegistry;
  __rasterReadMaterializeDiagnostics?: () => RasterFabricMaterializeDiagnostics;
  __rasterResetMaterializeDiagnostics?: () => void;
  __rasterRunEvent?: (
    handler: RasterHandler,
    payload: unknown,
    event: RasterEventMetadata | null
  ) => unknown;
};

const rasterGlobal = globalThis as RasterRuntimeGlobal;
const handlerRegistry =
  rasterGlobal.__rasterHandlerRegistry ??
  (rasterGlobal.__rasterHandlerRegistry = {
    handlers: new Map<HandlerId, HandlerRecord>(),
    handlerSlots: new Map<HandlerSlotKey, HandlerId>(),
  });
const handlers = handlerRegistry.handlers;
const handlerSlots = handlerRegistry.handlerSlots;
const RASTER_NATIVE_NODE_TYPE = "raster.native-node" as const;
const RASTER_NATIVE_CHILD_SET_TYPE = "raster.native-child-set" as const;
const RASTER_NATIVE_CONTAINER_TYPE = "raster.native-container" as const;
const fabricNodeChildHandles = new WeakMap<RasterNativeNode, RasterNativeNodeHandle[]>();
const fabricChildSetNodes = new WeakMap<RasterNativeChildSet, RasterNativeNode[]>();
const textControlEventCounts = new Map<string, number>();

const TEXT_EVENT_COUNT_PROP = "__rasterTextEventCount";

interface FabricShadowState {
  type: string | null;
  props: HostProps | null | undefined;
  key: string | null;
  text: string | null;
  hidden: boolean;
  childNodes: RasterNativeNode[];
  childHandles: RasterNativeNodeHandle[];
  handlerShapeKey: string;
  handlersDirty: boolean;
  propsDirty: boolean;
  childrenDirty: boolean;
  dirtyChildIndexes: Set<number>;
  committed: boolean;
}

const fabricNodeShadows = new WeakMap<RasterNativeNode, FabricShadowState>();

export interface RasterFabricMaterializeDiagnostics {
  lastVisitedNodes: number;
  lastEmittedNodes: number;
  lastCommittedRoots: number;
  totalVisitedNodes: number;
  totalEmittedNodes: number;
  totalCommittedRoots: number;
  commitCount: number;
}

interface RasterFabricMaterializeCommitCounters {
  visitedNodes: number;
  emittedNodes: number;
  committedRoots: number;
}

const fabricMaterializeDiagnostics: RasterFabricMaterializeDiagnostics = {
  lastVisitedNodes: 0,
  lastEmittedNodes: 0,
  lastCommittedRoots: 0,
  totalVisitedNodes: 0,
  totalEmittedNodes: 0,
  totalCommittedRoots: 0,
  commitCount: 0,
};

// Handler ids identify node-scoped event/query slots for the current JS
// runtime session; the slot record is updated on commit and ids are not reused.

function normalizeEventType(eventName: string): string {
  if (eventName.startsWith("on") && eventName.length > 2) {
    return eventName.slice(2).toLowerCase();
  }
  return eventName.toLowerCase();
}

export interface HandlerSlotDescriptor {
  surfaceId: RasterSurfaceId;
  nodeTag: RasterNodeTag;
  kind: HandlerKind;
  name: string;
  eventOrQueryType: string | null;
}

function handlerSlotKey(
  surfaceId: RasterSurfaceId,
  nodeTag: RasterNodeTag,
  kind: HandlerKind,
  name: string
): HandlerSlotKey {
  return `${surfaceId}:${nodeTag}:${kind}:${name}`;
}

function handlerSlotKeyForDescriptor(descriptor: HandlerSlotDescriptor): HandlerSlotKey {
  return handlerSlotKey(
    descriptor.surfaceId,
    descriptor.nodeTag,
    descriptor.kind,
    descriptor.name
  );
}

function textControlKey(surfaceId: RasterSurfaceId, nodeTag: RasterNodeTag): string {
  return `${surfaceId}:${nodeTag}`;
}

function isTextControlType(type: string): boolean {
  return type === "Input" || type === "Textarea";
}

function normalizeTextEventPayload(payload: unknown): { value: string; eventCount: number } | null {
  if (payload == null || typeof payload !== "object") {
    return null;
  }
  const record = payload as { value?: unknown; eventCount?: unknown };
  if (typeof record.value !== "string") {
    return null;
  }
  const eventCount = Number(record.eventCount);
  return {
    value: record.value,
    eventCount: Number.isFinite(eventCount) && eventCount >= 0 ? eventCount : 0,
  };
}

function recordTextControlEventCount(record: HandlerRecord, payload: unknown): void {
  if (record.name !== "onChange" && record.name !== "onChangeText") {
    return;
  }
  const textPayload = normalizeTextEventPayload(payload);
  if (textPayload == null) {
    return;
  }
  textControlEventCounts.set(
    textControlKey(record.surfaceId, record.nodeTag),
    textPayload.eventCount
  );
}

function userPayloadForHandler(record: HandlerRecord, payload: unknown): unknown {
  if (record.name !== "onChangeText") {
    return payload;
  }
  return normalizeTextEventPayload(payload)?.value ?? payload;
}

function updateHandlerRegistry(
  descriptor: HandlerSlotDescriptor,
  handler: unknown,
  createSlot: () => HandlerId,
  updateSlot?: (id: HandlerId, handler: RasterNativeJsFunctionRef) => void
): HandlerId | null {
  if (typeof handler !== "function") {
    return null;
  }

  const key = handlerSlotKeyForDescriptor(descriptor);
  let id = handlerSlots.get(key);
  let created = false;
  if (id == null) {
    id = createSlot();
    handlerSlots.set(key, id);
    created = true;
  }

  const eventType = descriptor.kind === "event" ? descriptor.eventOrQueryType : null;
  const jsFunctionRef = handler as RasterHandler;
  handlers.set(id, {
    handler: jsFunctionRef,
    eventType,
    surfaceId: descriptor.surfaceId,
    nodeTag: descriptor.nodeTag,
    name: descriptor.name,
  });
  if (created) {
    updateSlot?.(id, jsFunctionRef);
  }
  return id;
}

export function updateRasterHandlerSlot(
  descriptor: HandlerSlotDescriptor,
  handler: unknown,
  binding: RasterNativeBinding = getRasterNativeBinding()
): HandlerId | null {
  return updateHandlerRegistry(
    descriptor,
    handler,
    () =>
      binding.registerHandlerSlot(
        descriptor.surfaceId,
        descriptor.nodeTag,
        descriptor.kind,
        descriptor.name,
        descriptor.eventOrQueryType
      ),
    (id, jsFunctionRef) => {
      binding.updateHandlerSlot(id, jsFunctionRef);
    }
  );
}

export function dropRasterHandlerSlotsForNode(
  surfaceId: RasterSurfaceId,
  nodeTag: RasterNodeTag,
  binding?: RasterNativeBinding
): void {
  const prefix = `${surfaceId}:${nodeTag}:`;
  for (const [key, id] of Array.from(handlerSlots)) {
    if (key.startsWith(prefix)) {
      handlerSlots.delete(key);
      handlers.delete(id);
    }
  }
  binding?.dropHandlerSlotsForNode(surfaceId, nodeTag);
}

function normalizeJsonObject(value: object, path: string, label: string): JsonObject {
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) {
    throw new Error(`Unsupported ${label} at ${path}: expected a plain object`);
  }

  const object: JsonObject = {};
  for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
    object[key] = normalizeJsonValue(child, `${path}.${key}`, label);
  }
  return object;
}

function normalizeJsonValue(value: unknown, path: string, label: string): JsonValue {
  if (value === null) {
    return null;
  }

  if (typeof value === "boolean" || typeof value === "string") {
    return value;
  }
  if (typeof value === "number") {
    if (!Number.isFinite(value)) {
      throw new Error(`Unsupported ${label} at ${path}: expected a finite number`);
    }
    return value;
  }
  if (Array.isArray(value)) {
    return value.flatMap((child, index) =>
      child === undefined ? [] : [normalizeJsonValue(child, `${path}[${index}]`, label)]
    );
  }
  if (typeof value === "object") {
    return normalizeJsonObject(value as object, path, label);
  }

  throw new Error(`Unsupported ${label} at ${path}: expected JSON-like value`);
}

function createEventMetadata(eventType: string | null): RasterEventMetadata | null {
  if (eventType == null) {
    return null;
  }

  return {
    type: eventType,
    timeStamp: Date.now(),
  };
}

export function getRasterNativeBinding(): RasterNativeBinding {
  const binding = rasterGlobal.__rasterNative;
  if (binding == null) {
    throw new Error("Raster Fabric renderer requires globalThis.__rasterNative");
  }
  return binding;
}

export function createFabricContainer(
  options?: RasterRootOptions,
  binding: RasterNativeBinding = getRasterNativeBinding()
): RasterFabricContainer {
  return {
    $$typeof: RASTER_NATIVE_CONTAINER_TYPE,
    surfaceId: binding.createSurface(options ?? {}),
    parent: null,
  };
}

function nativeNodeDebug(name: string, key: string | null): RasterNativeNode["debug"] {
  return key == null ? { name } : { name, key };
}

function createNativeNodeWrapper(
  kind: RasterNativeNodeKind,
  handle: RasterNativeNodeHandle,
  debug: RasterNativeNode["debug"],
  childHandles: readonly RasterNativeNodeHandle[] = [],
  shadow?: Omit<FabricShadowState, "childHandles">
): RasterNativeNode {
  const node = {
    $$typeof: RASTER_NATIVE_NODE_TYPE,
    kind,
    tag: handle.node_tag,
    handle,
    debug,
  };
  fabricNodeChildHandles.set(node, [...childHandles]);
  if (shadow != null) {
    fabricNodeShadows.set(node, {
      ...shadow,
      childHandles: [...childHandles],
      dirtyChildIndexes: new Set(shadow.dirtyChildIndexes),
    });
  }
  return node;
}

export function readFabricChildHandles(instance: RasterNativeNode): RasterNativeNodeHandle[] {
  return [...(fabricNodeShadows.get(instance)?.childHandles ?? fabricNodeChildHandles.get(instance) ?? [])];
}

export function appendInitialFabricChild(
  parent: RasterNativeNode,
  child: RasterNativeNode,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.appendInitialChild(parent.handle, child.handle);
  const shadow = fabricNodeShadows.get(parent);
  if (shadow != null) {
    const index = shadow.childNodes.length;
    shadow.childNodes = [...shadow.childNodes, child];
    shadow.childrenDirty = true;
    shadow.dirtyChildIndexes.add(index);
    return;
  }

  const nextChildren = [...readFabricChildHandles(parent), child.handle];
  fabricNodeChildHandles.set(parent, nextChildren);
}

function nativeKindForHostType(type: string): RasterNativeNodeKind {
  switch (type) {
    case "Input":
      return "input";
    case "Textarea":
      return "textarea";
    case "Label":
    case "Widget":
      return "widget";
    case "Slot":
      return "slot";
    case "ConfigProvider":
      return "config_provider";
    default:
      return "host";
  }
}

const fabricReservedProps = new Set(["children", "key", "ref", "style", "queries"]);

function normalizeFabricProps(type: string, props: HostProps | null | undefined): JsonObject {
  const result: JsonObject = {};
  for (const [key, value] of Object.entries(props ?? {})) {
    if (fabricReservedProps.has(key) || value === undefined || typeof value === "function") {
      continue;
    }
    result[key] = normalizeJsonValue(value, `props.${key}`, `${type} prop`);
  }
  return result;
}

function normalizeFabricStyle(type: string, props: HostProps | null | undefined): JsonValue {
  return normalizeJsonValue(props?.style ?? {}, "style", `${type} style`);
}

function normalizeFabricContext(type: string, props: HostProps | null | undefined): JsonObject | undefined {
  if (type !== "ConfigProvider") {
    return undefined;
  }
  return {
    theme: normalizeJsonValue(props?.theme ?? {}, "theme", "ConfigProvider context"),
    text: normalizeJsonValue(props?.text ?? {}, "text", "ConfigProvider context"),
    resources: normalizeJsonValue(props?.resources ?? {}, "resources", "ConfigProvider context"),
  };
}

function normalizeFabricBasePayload(
  type: string,
  props: HostProps | null | undefined,
  hidden = false
): RasterShadowNodePayload {
  const payload: RasterShadowNodePayload = {
    props: normalizeFabricProps(type, props),
    style: normalizeFabricStyle(type, props),
    hidden,
    event_bindings: [],
    query_bindings: [],
  };
  const context = normalizeFabricContext(type, props);
  if (context != null) {
    payload.context = context;
  }
  return payload;
}

function attachTextControlEventCount(
  payload: RasterShadowNodePayload,
  surfaceId: RasterSurfaceId,
  nodeTag: RasterNodeTag,
  type: string,
  props: HostProps | null | undefined
): RasterShadowNodePayload {
  if (!isTextControlType(type) || props?.value == null) {
    return payload;
  }
  payload.props = {
    ...(payload.props ?? {}),
    [TEXT_EVENT_COUNT_PROP]: textControlEventCounts.get(textControlKey(surfaceId, nodeTag)) ?? 0,
  };
  return payload;
}

function normalizeFabricTextPayload(text: unknown, hidden = false): RasterShadowNodePayload {
  return {
    props: {},
    style: {},
    text: String(text),
    hidden,
  };
}

function collectFabricHandlers(props: HostProps | null | undefined): Array<{
  kind: HandlerKind;
  name: string;
  eventOrQueryType: string | null;
  handler: unknown;
}> {
  const handlersToBind: Array<{
    kind: HandlerKind;
    name: string;
    eventOrQueryType: string | null;
    handler: unknown;
  }> = [];

  for (const [key, value] of Object.entries(props ?? {})) {
    if (key === "children" || key === "key" || key === "ref" || value == null) {
      continue;
    }

    if (key === "queries") {
      if (Array.isArray(value) || typeof value !== "object") {
        throw new Error("Fabric queries must be an object");
      }
      for (const [queryName, queryHandler] of Object.entries(value as Record<string, unknown>)) {
        if (queryHandler == null) {
          continue;
        }
        if (typeof queryHandler !== "function") {
          throw new Error(`Fabric query "${queryName}" must be a function`);
        }
        handlersToBind.push({
          kind: "query",
          name: queryName,
          eventOrQueryType: queryName,
          handler: queryHandler,
        });
      }
      continue;
    }

    if (typeof value !== "function") {
      continue;
    }

    if (key.startsWith("on")) {
      handlersToBind.push({
        kind: "event",
        name: key,
        eventOrQueryType: normalizeEventType(key),
        handler: value,
      });
    } else {
      handlersToBind.push({
        kind: "query",
        name: key,
        eventOrQueryType: key,
        handler: value,
      });
    }
  }

  return handlersToBind;
}

function hasFabricHandlers(props: HostProps | null | undefined): boolean {
  return collectFabricHandlers(props).length > 0;
}

function fabricHandlerShapeKey(props: HostProps | null | undefined): string {
  const handlers = collectFabricHandlers(props)
    .map((handler) => ({
      kind: handler.kind,
      name: handler.name,
      eventOrQueryType: handler.eventOrQueryType,
    }))
    .sort((left, right) => {
      const leftKey = `${left.kind}:${left.name}:${left.eventOrQueryType ?? ""}`;
      const rightKey = `${right.kind}:${right.name}:${right.eventOrQueryType ?? ""}`;
      return leftKey.localeCompare(rightKey);
    });
  return stableJsonStringify(handlers as JsonValue);
}

function stableJsonStringify(value: JsonValue | undefined): string {
  if (value == null || typeof value !== "object") {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((child) => stableJsonStringify(child)).join(",")}]`;
  }

  const entries = Object.entries(value).sort(([left], [right]) => left.localeCompare(right));
  return `{${entries
    .map(([key, child]) => `${JSON.stringify(key)}:${stableJsonStringify(child)}`)
    .join(",")}}`;
}

function fabricBasePayloadKey(
  type: string,
  props: HostProps | null | undefined,
  hidden: boolean
): string {
  const payload = normalizeFabricBasePayload(type, props, hidden);
  return stableJsonStringify(payload as JsonObject);
}

function shouldDirtyFabricHostProps(
  type: string,
  oldProps: HostProps | null | undefined,
  newProps: HostProps | null | undefined,
  oldHidden: boolean,
  newHidden: boolean,
  oldHandlerShapeKey: string
): boolean {
  return (
    oldHidden !== newHidden ||
    fabricBasePayloadKey(type, oldProps, oldHidden) !==
      fabricBasePayloadKey(type, newProps, newHidden) ||
    oldHandlerShapeKey !== fabricHandlerShapeKey(newProps)
  );
}

function attachFabricHandlerBindings(
  payload: RasterShadowNodePayload,
  surfaceId: RasterSurfaceId,
  nodeTag: RasterNodeTag,
  props: HostProps | null | undefined,
  binding: RasterNativeBinding
): RasterShadowNodePayload {
  const events: RasterNativeEventBinding[] = [];
  const queries: RasterNativeQueryBinding[] = [];

  for (const handler of collectFabricHandlers(props)) {
    const slotId = updateRasterHandlerSlot(
      {
        surfaceId,
        nodeTag,
        kind: handler.kind,
        name: handler.name,
        eventOrQueryType: handler.eventOrQueryType,
      },
      handler.handler,
      binding
    );
    if (slotId == null) {
      continue;
    }
    if (handler.kind === "event") {
      events.push({
        property: handler.name,
        event_type: handler.eventOrQueryType,
        handler_slot_id: slotId,
      });
    } else {
      queries.push({
        property: handler.name,
        query_type: handler.eventOrQueryType,
        handler_slot_id: slotId,
      });
    }
  }

  payload.event_bindings = events;
  payload.query_bindings = queries;
  return payload;
}

function normalizeFabricPayloadForNode(
  surfaceId: RasterSurfaceId,
  nodeTag: RasterNodeTag,
  type: string,
  props: HostProps | null | undefined,
  hidden: boolean,
  binding: RasterNativeBinding
): RasterShadowNodePayload {
  return attachTextControlEventCount(
    attachFabricHandlerBindings(
      normalizeFabricBasePayload(type, props, hidden),
      surfaceId,
      nodeTag,
      props,
      binding
    ),
    surfaceId,
    nodeTag,
    type,
    props,
  );
}

export function createFabricHostNode(
  surfaceId: RasterSurfaceId,
  type: string,
  props: HostProps | null | undefined,
  key: string | null = null,
  binding: RasterNativeBinding = getRasterNativeBinding()
): RasterNativeNode {
  const kind = nativeKindForHostType(type);
  const initialPayload = normalizeFabricBasePayload(type, props, false);
  const handle = binding.createNode(
    surfaceId,
    kind,
    type,
    key == null ? null : String(key),
    initialPayload
  );
  binding.updateNode(
    handle,
    normalizeFabricPayloadForNode(surfaceId, handle.node_tag, type, props, false, binding)
  );

  return createNativeNodeWrapper(kind, handle, nativeNodeDebug(type, key), [], {
    type,
    props,
    key,
    text: null,
    hidden: false,
    childNodes: [],
    handlerShapeKey: "",
    handlersDirty: hasFabricHandlers(props),
    propsDirty: hasFabricHandlers(props),
    childrenDirty: false,
    dirtyChildIndexes: new Set(),
    committed: true,
  });
}

export function createFabricTextNode(
  surfaceId: RasterSurfaceId,
  text: unknown,
  binding: RasterNativeBinding = getRasterNativeBinding()
): RasterNativeNode {
  const nextText = String(text);
  const handle = binding.createTextNode(surfaceId, nextText, normalizeFabricTextPayload(nextText));
  return createNativeNodeWrapper("text", handle, { name: "Text" }, [], {
    type: null,
    props: null,
    key: null,
    text: nextText,
    hidden: false,
    childNodes: [],
    handlerShapeKey: "",
    handlersDirty: false,
    propsDirty: false,
    childrenDirty: false,
    dirtyChildIndexes: new Set(),
    committed: true,
  });
}

export function prepareFabricCommit(
  container: RasterFabricContainer,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.prepareForCommit(container.surfaceId);
}

export function resetFabricCommit(
  container: RasterFabricContainer,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.resetAfterCommit(container.surfaceId);
}

export function clearFabricSurface(
  container: RasterFabricContainer,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.clearSurface?.(container.surfaceId);
}

export function appendFabricChild(
  parent: RasterNativeNode,
  child: RasterNativeNode,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.appendChild(parent.handle, child.handle);
}

export function appendFabricChildToContainer(
  container: RasterFabricContainer,
  child: RasterNativeNode,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.appendChildToContainer(container.surfaceId, child.handle);
}

export function insertFabricChildBefore(
  parent: RasterNativeNode,
  child: RasterNativeNode,
  before: RasterNativeNode,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.insertBefore(parent.handle, child.handle, before.handle);
}

export function insertFabricChildInContainerBefore(
  container: RasterFabricContainer,
  child: RasterNativeNode,
  before: RasterNativeNode,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.insertInContainerBefore(container.surfaceId, child.handle, before.handle);
}

export function removeFabricChild(
  parent: RasterNativeNode,
  child: RasterNativeNode,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.removeChild(parent.handle, child.handle);
}

export function removeFabricChildFromContainer(
  container: RasterFabricContainer,
  child: RasterNativeNode,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.removeChildFromContainer(container.surfaceId, child.handle);
}

export function updateFabricHostNode(
  instance: RasterNativeNode,
  type: string,
  props: HostProps | null | undefined,
  hidden = false,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.updateNode(
    instance.handle,
    normalizeFabricPayloadForNode(
      instance.handle.surface_id,
      instance.handle.node_tag,
      type,
      props,
      hidden,
      binding
    )
  );
}

export function updateFabricTextNode(
  instance: RasterNativeNode,
  text: unknown,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.updateTextNode(instance.handle, String(text));
}

export function cloneFabricHostNode(
  instance: RasterNativeNode,
  type: string,
  _oldProps: HostProps | null | undefined,
  newProps: HostProps | null | undefined,
  hidden = false,
  binding: RasterNativeBinding = getRasterNativeBinding(),
  keepChildren = true
): RasterNativeNode {
  void _oldProps;
  void binding;
  const previousShadow = fabricNodeShadows.get(instance);
  const childNodes = keepChildren ? [...(previousShadow?.childNodes ?? [])] : [];
  const dirtyChildIndexes = new Set<number>();
  if (keepChildren) {
    childNodes.forEach((child, index) => {
      if (fabricNodeNeedsMaterializeVisit(child)) {
        dirtyChildIndexes.add(index);
      }
    });
  }
  const childHandles = readFabricChildHandles(instance);
  const previousHidden = previousShadow?.hidden ?? false;
  const previousHandlerShapeKey = previousShadow?.handlerShapeKey ?? fabricHandlerShapeKey(_oldProps);
  return createNativeNodeWrapper(
    instance.kind,
    instance.handle,
    nativeNodeDebug(type, instance.debug?.key ?? null),
    childHandles,
    {
      type,
      props: newProps,
      key: instance.debug?.key ?? null,
      text: null,
      hidden,
      childNodes,
      handlerShapeKey: previousHandlerShapeKey,
      handlersDirty: hasFabricHandlers(newProps),
      propsDirty: shouldDirtyFabricHostProps(
        type,
        _oldProps,
        newProps,
        previousHidden,
        hidden,
        previousHandlerShapeKey
      ),
      childrenDirty: !keepChildren || dirtyChildIndexes.size > 0,
      dirtyChildIndexes,
      committed: previousShadow?.committed ?? true,
    }
  );
}

export function cloneFabricTextNode(
  instance: RasterNativeNode,
  text: unknown,
  hidden = false,
  binding: RasterNativeBinding = getRasterNativeBinding()
): RasterNativeNode {
  const nextText = String(text);
  void binding;
  const previousText = fabricNodeShadows.get(instance)?.text;
  const previousHidden = fabricNodeShadows.get(instance)?.hidden ?? false;
  const previousCommitted = fabricNodeShadows.get(instance)?.committed ?? true;
  return createNativeNodeWrapper("text", instance.handle, instance.debug ?? { name: "Text" }, [], {
    type: null,
    props: null,
    key: null,
    text: nextText,
    hidden,
    childNodes: [],
    handlerShapeKey: "",
    handlersDirty: false,
    propsDirty: previousText !== nextText || previousHidden !== hidden,
    childrenDirty: false,
    dirtyChildIndexes: new Set(),
    committed: previousCommitted,
  });
}

type FabricMaterializeFrame = {
  spec: RasterNativeMaterializeSpec;
  node: RasterNativeNode;
  handleMayChange: boolean;
};

function handlesAreEqual(a: RasterNativeNodeHandle, b: RasterNativeNodeHandle): boolean {
  return (
    a.surface_id === b.surface_id &&
    a.node_tag === b.node_tag &&
    a.revision_id === b.revision_id &&
    a.generation === b.generation
  );
}

interface FabricMaterializeBuildContext {
  binding: RasterNativeBinding;
  counters: RasterFabricMaterializeCommitCounters;
  dirtyNodes: Set<RasterNativeNode>;
}

function recordFabricMaterializeCommit(counters: RasterFabricMaterializeCommitCounters): void {
  fabricMaterializeDiagnostics.lastVisitedNodes = counters.visitedNodes;
  fabricMaterializeDiagnostics.lastEmittedNodes = counters.emittedNodes;
  fabricMaterializeDiagnostics.lastCommittedRoots = counters.committedRoots;
  fabricMaterializeDiagnostics.totalVisitedNodes += counters.visitedNodes;
  fabricMaterializeDiagnostics.totalEmittedNodes += counters.emittedNodes;
  fabricMaterializeDiagnostics.totalCommittedRoots += counters.committedRoots;
  fabricMaterializeDiagnostics.commitCount += 1;
}

export function readRasterFabricMaterializeDiagnostics(): RasterFabricMaterializeDiagnostics {
  return { ...fabricMaterializeDiagnostics };
}

export function resetRasterFabricMaterializeDiagnostics(): void {
  fabricMaterializeDiagnostics.lastVisitedNodes = 0;
  fabricMaterializeDiagnostics.lastEmittedNodes = 0;
  fabricMaterializeDiagnostics.lastCommittedRoots = 0;
  fabricMaterializeDiagnostics.totalVisitedNodes = 0;
  fabricMaterializeDiagnostics.totalEmittedNodes = 0;
  fabricMaterializeDiagnostics.totalCommittedRoots = 0;
  fabricMaterializeDiagnostics.commitCount = 0;
}

function fabricNodeNeedsMaterializeVisit(instance: RasterNativeNode): boolean {
  const shadow = fabricNodeShadows.get(instance);
  return (
    shadow != null &&
    (!shadow.committed ||
      shadow.propsDirty ||
      shadow.handlersDirty ||
      fabricChildrenNeedMaterialize(shadow))
  );
}

function fabricChildrenNeedMaterialize(shadow: FabricShadowState): boolean {
  if (!shadow.childrenDirty && shadow.dirtyChildIndexes.size === 0) {
    return false;
  }
  if (shadow.childNodes.length !== shadow.childHandles.length) {
    return true;
  }

  return shadow.childNodes.some((child, index) => {
    const previousHandle = shadow.childHandles[index];
    return (
      previousHandle == null ||
      !handlesAreEqual(child.handle, previousHandle) ||
      fabricNodeNeedsMaterializeVisit(child)
    );
  });
}

function buildFabricShallowMaterializeFrame(
  instance: RasterNativeNode,
  context: FabricMaterializeBuildContext
): FabricMaterializeFrame {
  context.counters.emittedNodes += 1;
  return {
    spec: { handle: instance.handle },
    node: instance,
    handleMayChange: false,
  };
}

function buildFabricMaterializeFrame(
  instance: RasterNativeNode,
  context: FabricMaterializeBuildContext
): FabricMaterializeFrame {
  const frames = new WeakMap<RasterNativeNode, FabricMaterializeFrame>();
  const stack: Array<{ node: RasterNativeNode; visited: boolean }> = [
    { node: instance, visited: false },
  ];

  while (stack.length > 0) {
    const frame = stack.pop()!;
    if (!fabricNodeNeedsMaterializeVisit(frame.node)) {
      frames.set(frame.node, buildFabricShallowMaterializeFrame(frame.node, context));
      continue;
    }

    const shadow = fabricNodeShadows.get(frame.node);
    if (shadow == null) {
      frames.set(frame.node, buildFabricShallowMaterializeFrame(frame.node, context));
      continue;
    }

    if (!frame.visited) {
      stack.push({ node: frame.node, visited: true });
      if (frame.node.kind !== "text" && fabricChildrenNeedMaterialize(shadow)) {
        for (let index = shadow.childNodes.length - 1; index >= 0; index -= 1) {
          const child = shadow.childNodes[index]!;
          if (fabricNodeNeedsMaterializeVisit(child) && !frames.has(child)) {
            stack.push({ node: child, visited: false });
          }
        }
      }
      continue;
    }

    frames.set(frame.node, buildFabricMaterializeFrameFromChildren(frame.node, shadow, context, frames));
  }

  return (
    frames.get(instance) ?? {
      spec: { handle: instance.handle },
      node: instance,
      handleMayChange: false,
    }
  );
}

function buildFabricMaterializeFrameFromChildren(
  instance: RasterNativeNode,
  shadow: FabricShadowState,
  context: FabricMaterializeBuildContext,
  frames: WeakMap<RasterNativeNode, FabricMaterializeFrame>
): FabricMaterializeFrame {
  context.counters.emittedNodes += 1;
  context.counters.visitedNodes += 1;
  context.dirtyNodes.add(instance);

  let spec: RasterNativeMaterializeSpec = { handle: instance.handle };
  let handleMayChange = false;

  if (instance.kind === "text") {
    if (!shadow.committed) {
      const nextText = shadow.text ?? "";
      spec = {
        kind: "text",
        text: nextText,
        payload: normalizeFabricTextPayload(nextText, shadow.hidden),
      };
      handleMayChange = true;
    } else if (shadow.propsDirty) {
      const nextText = shadow.text ?? "";
      spec = {
        handle: instance.handle,
        payload: normalizeFabricTextPayload(nextText, shadow.hidden),
      };
      handleMayChange = true;
    }
    return { spec, node: instance, handleMayChange };
  }

  const childrenNeedMaterialize = fabricChildrenNeedMaterialize(shadow);
  const childUpdates: RasterNativeMaterializeChildUpdate[] = [];
  const useFullChildren = childrenNeedMaterialize && shadow.childHandles.length === 0;
  const childFrames = childrenNeedMaterialize
    ? shadow.childNodes.map((child, index) => {
        if (!useFullChildren && !fabricNodeNeedsMaterializeVisit(child)) {
          return null;
        }
        const childFrame = frames.get(child) ?? buildFabricShallowMaterializeFrame(child, context);
        if (!useFullChildren) {
          childUpdates.push({ index, child: childFrame.spec });
        }
        return childFrame;
      })
    : [];
  const childHandleMayChange = childFrames.some((child) => child?.handleMayChange);
  const needsChildren = childrenNeedMaterialize || childHandleMayChange;
  const type = shadow.type ?? instance.debug?.name ?? "View";

  if (shadow.handlersDirty && !shadow.propsDirty) {
    attachFabricHandlerBindings(
      {},
      instance.handle.surface_id,
      instance.handle.node_tag,
      shadow.props,
      context.binding
    );
  }

  const payload = shadow.propsDirty
    ? normalizeFabricPayloadForNode(
        instance.handle.surface_id,
        instance.handle.node_tag,
        type,
        shadow.props,
        shadow.hidden,
        context.binding
      )
    : {};
  if (shadow.propsDirty || needsChildren) {
    spec = {
      handle: instance.handle,
      payload,
    };
    if (needsChildren) {
      if (useFullChildren) {
        spec.children = childFrames
          .filter((child): child is FabricMaterializeFrame => child != null)
          .map((child) => child.spec);
      } else {
        spec.childHandles = shadow.childNodes.map((child) => child.handle);
        if (childUpdates.length > 0) {
          spec.childUpdates = childUpdates;
        }
      }
    }
    handleMayChange = true;
  }

  return { spec, node: instance, handleMayChange };
}

function markFabricNodeMaterialized(instance: RasterNativeNode): void {
  const shadow = fabricNodeShadows.get(instance);
  if (shadow == null) {
    return;
  }
  shadow.handlersDirty = false;
  shadow.propsDirty = false;
  shadow.childrenDirty = false;
  shadow.dirtyChildIndexes.clear();
  shadow.committed = true;
  shadow.handlerShapeKey = fabricHandlerShapeKey(shadow.props);
  const nextChildHandles = shadow.childNodes.map((child) => child.handle);
  shadow.childHandles = nextChildHandles;
  fabricNodeChildHandles.set(instance, nextChildHandles);
}

function applyFabricMaterializeResult(
  instance: RasterNativeNode,
  result: RasterNativeMaterializeResult
): void {
  instance.handle = result.handle;
  instance.tag = result.handle.node_tag;
  const shadow = fabricNodeShadows.get(instance);
  if (shadow == null) {
    return;
  }
  const childUpdates = result.childUpdates ?? [];
  for (const childUpdate of childUpdates) {
    const child = shadow.childNodes[childUpdate.index];
    if (child != null) {
      applyFabricMaterializeResult(child, childUpdate.child);
    }
  }
  for (let index = 0; index < result.children.length; index += 1) {
    const child = shadow.childNodes[index];
    const childResult = result.children[index];
    if (child != null && childResult != null) {
      applyFabricMaterializeResult(child, childResult);
    }
  }
  const nextChildHandles = shadow.childNodes.map((child) => child.handle);
  shadow.propsDirty = false;
  shadow.handlersDirty = false;
  shadow.childrenDirty = false;
  shadow.dirtyChildIndexes.clear();
  shadow.committed = true;
  shadow.handlerShapeKey = fabricHandlerShapeKey(shadow.props);
  shadow.childHandles = nextChildHandles;
  fabricNodeChildHandles.set(instance, nextChildHandles);
}

function commitFabricNodesToSurface(
  surfaceId: RasterSurfaceId,
  children: RasterNativeNode[],
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  const counters: RasterFabricMaterializeCommitCounters = {
    visitedNodes: 0,
    emittedNodes: 0,
    committedRoots: children.length,
  };
  const context: FabricMaterializeBuildContext = {
    binding,
    counters,
    dirtyNodes: new Set(),
  };
  const frames = children.map((child) => buildFabricMaterializeFrame(child, context));
  const results = binding.commitSurfaceTree(
    surfaceId,
    frames.map((frame) => frame.spec)
  );
  for (let index = 0; index < results.length; index += 1) {
    const child = children[index];
    const result = results[index];
    if (child != null && result != null) {
      applyFabricMaterializeResult(child, result);
    }
  }
  for (const child of context.dirtyNodes) {
    markFabricNodeMaterialized(child);
  }
  recordFabricMaterializeCommit(counters);
}

function clearFabricChildSet(childSet: RasterNativeChildSet): RasterNativeNode[] {
  const children = fabricChildSetNodes.get(childSet) ?? [];
  fabricChildSetNodes.delete(childSet);
  return children;
}

function createSyntheticChildSetHandle(surfaceId: RasterSurfaceId): RasterNativeChildSet["handle"] {
  return {
    surface_id: surfaceId,
    child_set_id: 0,
    generation: 0,
  };
}

export function createFabricChildSet(
  container: RasterFabricContainer,
  binding: RasterNativeBinding = getRasterNativeBinding()
): RasterNativeChildSet {
  void binding;
  const childSet = {
    $$typeof: RASTER_NATIVE_CHILD_SET_TYPE,
    surfaceId: container.surfaceId,
    handle: createSyntheticChildSetHandle(container.surfaceId),
  } as RasterNativeChildSet;
  fabricChildSetNodes.set(childSet, []);
  return childSet;
}

export function appendFabricChildToSet(
  childSet: RasterNativeChildSet,
  child: RasterNativeNode,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  void binding;
  const children = fabricChildSetNodes.get(childSet);
  if (children == null) {
    throw new Error("Cannot append to an unknown Raster Fabric child set");
  }
  children.push(child);
}

export function finalizeFabricChildSet(
  childSet: RasterNativeChildSet,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  void childSet;
  void binding;
}

export function commitFabricChildSet(
  container: RasterFabricContainer,
  childSet: RasterNativeChildSet,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  commitFabricNodesToSurface(container.surfaceId, clearFabricChildSet(childSet), binding);
}

export function deleteFabricNode(
  instance: RasterNativeNode,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  dropRasterHandlerSlotsForNode(instance.handle.surface_id, instance.handle.node_tag, binding);
  binding.deleteNode(instance.handle);
}

rasterGlobal.__rasterInvokeHandler = function (id: HandlerId, payload: unknown): unknown {
  const record = handlers.get(id);
  if (!record) {
    return undefined;
  }

  recordTextControlEventCount(record, payload);
  const userPayload = userPayloadForHandler(record, payload);
  const result =
    typeof rasterGlobal.__rasterRunEvent === "function"
      ? rasterGlobal.__rasterRunEvent(
          record.handler,
          userPayload,
          createEventMetadata(record.eventType)
        )
      : record.handler(userPayload);
  if (typeof rasterGlobal.__rasterFlushSyncWork === "function") {
    rasterGlobal.__rasterFlushSyncWork();
  }
  return result;
};

rasterGlobal.__rasterInvokeHandlerJson = function (
  id: HandlerId,
  payloadJson: string
): unknown {
  return rasterGlobal.__rasterInvokeHandler?.(id, JSON.parse(payloadJson));
};

rasterGlobal.__rasterInvokeHandlersJson = function (callsJson: string): unknown[] {
  const calls = JSON.parse(callsJson) as Array<{ id: HandlerId; payload: unknown }>;
  const resolvedCalls = calls.map((call) => ({
    call,
    record: handlers.get(call.id),
  }));
  const results: unknown[] = [];

  for (const { call, record } of resolvedCalls) {
    if (!record) {
      results.push(undefined);
      continue;
    }

    recordTextControlEventCount(record, call.payload);
    const userPayload = userPayloadForHandler(record, call.payload);
    results.push(
      typeof rasterGlobal.__rasterRunEvent === "function"
        ? rasterGlobal.__rasterRunEvent(
            record.handler,
            userPayload,
            createEventMetadata(record.eventType)
          )
        : record.handler(userPayload)
    );
  }

  if (typeof rasterGlobal.__rasterFlushSyncWork === "function") {
    rasterGlobal.__rasterFlushSyncWork();
  }

  return results;
};

rasterGlobal.__rasterInvokeQuery = function (id: HandlerId, payload: unknown): unknown {
  const record = handlers.get(id);
  if (!record) {
    return undefined;
  }

  return record.handler(payload);
};

rasterGlobal.__rasterInvokeQueryJson = function (
  id: HandlerId,
  payloadJson: string
): string {
  const result = rasterGlobal.__rasterInvokeQuery?.(id, JSON.parse(payloadJson));
  return JSON.stringify(result ?? null);
};

rasterGlobal.__rasterReadMaterializeDiagnostics = readRasterFabricMaterializeDiagnostics;
rasterGlobal.__rasterResetMaterializeDiagnostics = resetRasterFabricMaterializeDiagnostics;
