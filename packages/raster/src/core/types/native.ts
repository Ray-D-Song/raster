import type { JsonObject, JsonValue } from "./json.js";
import type {
  RasterHandlerSlotId,
  RasterHandlerSlotKind,
  RasterNativeChildSetId,
  RasterNodeTag,
  RasterShadowRevisionId,
  RasterSurfaceGeneration,
  RasterSurfaceId,
} from "./events.js";

export interface RasterRootOptions {
  width?: number | null;
  height?: number | null;
  perfdetect?: boolean | null;
}


export type RasterNativeNodeKind =
  | "host"
  | "text"
  | "input"
  | "textarea"
  | "widget"
  | "fragment"
  | "slot"
  | "config_provider";


export interface RasterNativeNodeHandle {
  surface_id: RasterSurfaceId;
  node_tag: RasterNodeTag;
  revision_id: RasterShadowRevisionId;
  generation: RasterSurfaceGeneration;
}

export interface RasterNativeChildSetHandle {
  surface_id: RasterSurfaceId;
  child_set_id: RasterNativeChildSetId;
  generation: RasterSurfaceGeneration;
}

export interface RasterNativeNode {
  $$typeof: "raster.native-node";
  kind: RasterNativeNodeKind;
  tag: RasterNodeTag;
  handle: RasterNativeNodeHandle;
  debug?: {
    name?: string;
    key?: string;
    componentStack?: string;
  };
}

export interface RasterNativeChildSet {
  $$typeof: "raster.native-child-set";
  surfaceId: RasterSurfaceId;
  handle: RasterNativeChildSetHandle;
}

export interface RasterNativeEventBinding {
  property: string;
  event_type: string | null;
  options?: JsonObject;
  handler_slot_id: RasterHandlerSlotId;
}

export interface RasterNativeQueryBinding {
  property: string;
  query_type: string | null;
  options?: JsonObject;
  handler_slot_id: RasterHandlerSlotId;
}

export interface RasterShadowNodePayload {
  props?: JsonObject;
  style?: JsonValue;
  text?: string;
  hidden?: boolean;
  context?: JsonObject;
  event_bindings?: RasterNativeEventBinding[];
  query_bindings?: RasterNativeQueryBinding[];
}

export type RasterShadowNodeUpdatePayload = RasterShadowNodePayload;

export interface RasterNativeMaterializeSpec {
  handle?: RasterNativeNodeHandle;
  kind?: RasterNativeNodeKind;
  name?: string;
  key?: string | null;
  text?: string;
  payload?: RasterShadowNodeUpdatePayload;
  children?: RasterNativeMaterializeSpec[];
  childHandles?: RasterNativeNodeHandle[];
  childUpdates?: RasterNativeMaterializeChildUpdate[];
}

export interface RasterNativeMaterializeChildUpdate {
  index: number;
  child: RasterNativeMaterializeSpec;
}

export interface RasterNativeMaterializeResult {
  handle: RasterNativeNodeHandle;
  children: RasterNativeMaterializeResult[];
  childUpdates?: RasterNativeMaterializeChildResult[];
}

export interface RasterNativeMaterializeChildResult {
  index: number;
  child: RasterNativeMaterializeResult;
}

export type RasterNativeJsFunctionRef = (payload: unknown) => unknown;

export interface RasterNativeBinding {
  createSurface(options?: RasterRootOptions): RasterSurfaceId;
  createNode(
    surfaceId: RasterSurfaceId,
    kind: RasterNativeNodeKind,
    name: string,
    key: string | null,
    payload: RasterShadowNodePayload
  ): RasterNativeNodeHandle;
  createTextNode(
    surfaceId: RasterSurfaceId,
    text: string,
    payload: RasterShadowNodePayload
  ): RasterNativeNodeHandle;
  appendInitialChild(parent: RasterNativeNodeHandle, child: RasterNativeNodeHandle): void;
  prepareForCommit(surfaceId: RasterSurfaceId): void;
  resetAfterCommit(surfaceId: RasterSurfaceId): void;
  clearSurface?(surfaceId: RasterSurfaceId): void;
  appendChild(parent: RasterNativeNodeHandle, child: RasterNativeNodeHandle): void;
  appendChildToContainer(surfaceId: RasterSurfaceId, child: RasterNativeNodeHandle): void;
  insertBefore(
    parent: RasterNativeNodeHandle,
    child: RasterNativeNodeHandle,
    before: RasterNativeNodeHandle
  ): void;
  insertInContainerBefore(
    surfaceId: RasterSurfaceId,
    child: RasterNativeNodeHandle,
    before: RasterNativeNodeHandle
  ): void;
  removeChild(parent: RasterNativeNodeHandle, child: RasterNativeNodeHandle): void;
  removeChildFromContainer(surfaceId: RasterSurfaceId, child: RasterNativeNodeHandle): void;
  updateNode(handle: RasterNativeNodeHandle, payload: RasterShadowNodeUpdatePayload): void;
  updateTextNode(handle: RasterNativeNodeHandle, text: string): void;
  cloneNode(
    handle: RasterNativeNodeHandle,
    payload: RasterShadowNodeUpdatePayload
  ): RasterNativeNodeHandle;
  cloneNodeWithChildren(
    handle: RasterNativeNodeHandle,
    payload: RasterShadowNodeUpdatePayload,
    children: RasterNativeNodeHandle[]
  ): RasterNativeNodeHandle;
  createChildSet(surfaceId: RasterSurfaceId): RasterNativeChildSetHandle;
  appendChildToSet(childSet: RasterNativeChildSetHandle, childHandle: RasterNativeNodeHandle): void;
  finalizeChildSet(childSet: RasterNativeChildSetHandle): void;
  commitChildSet(surfaceId: RasterSurfaceId, childSet: RasterNativeChildSetHandle): void;
  commitSurfaceTree(
    surfaceId: RasterSurfaceId,
    roots: RasterNativeMaterializeSpec[]
  ): RasterNativeMaterializeResult[];
  deleteNode(handle: RasterNativeNodeHandle): void;
  registerHandlerSlot(
    surfaceId: RasterSurfaceId,
    nodeTag: RasterNodeTag,
    kind: RasterHandlerSlotKind,
    property: string,
    eventOrQueryType: string | null
  ): RasterHandlerSlotId;
  updateHandlerSlot(handlerSlotId: RasterHandlerSlotId, jsFunctionRef: RasterNativeJsFunctionRef): void;
  dropHandlerSlotsForNode(surfaceId: RasterSurfaceId, nodeTag: RasterNodeTag): void;
  notificationShow?(options: RasterNotificationShowOptions): void;
  notificationDismiss?(id: string): void;
  notificationClear?(): void;
  chartAppendData?(handle: RasterNativeNodeHandle, rows: JsonValue[]): void;
  chartReplaceData?(handle: RasterNativeNodeHandle, rows: JsonValue[]): void;
  chartClearData?(handle: RasterNativeNodeHandle): void;
  getTheme?(): string;
}

export type RasterNotificationType = "info" | "success" | "warning" | "error";

export interface RasterNotificationShowOptions {
  id?: string;
  type?: RasterNotificationType;
  title?: string;
  message: string;
  autohide?: boolean;
}

export type NativeNodeHandle = RasterNativeNodeHandle;
export type NativeChildSetHandle = RasterNativeChildSetHandle;

