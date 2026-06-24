import type {
  JsonObject,
  JsonValue,
  RasterNativeBinding,
  RasterNativeEventBinding,
  RasterNativeQueryBinding,
  RasterNodeTag,
  RasterShadowNodePayload,
  RasterSurfaceId,
} from "../types/index.js";
import { collectFabricHandlers, updateRasterHandlerSlot } from "../events/index.js";
import { isTextControlType, readTextControlEventCount, TEXT_EVENT_COUNT_PROP } from "../events/text-control.js";

type HostProps = Record<string, unknown>;

function normalizeJsonObject(value: object, path: string, label: string): JsonObject {
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) {
    throw new Error("Unsupported " + label + " at " + path + ": expected a plain object");
  }

  const object: JsonObject = {};
  for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
    if (child === undefined) {
      continue;
    }
    object[key] = normalizeJsonValue(child, path + "." + key, label);
  }
  return object;
}

export function normalizeJsonValue(value: unknown, path: string, label: string): JsonValue {
  if (value === null) {
    return null;
  }

  if (typeof value === "boolean" || typeof value === "string") {
    return value;
  }
  if (typeof value === "number") {
    if (!Number.isFinite(value)) {
      throw new Error("Unsupported " + label + " at " + path + ": expected a finite number");
    }
    return value;
  }
  if (Array.isArray(value)) {
    return value.flatMap((child, index) =>
      child === undefined ? [] : [normalizeJsonValue(child, path + "[" + index + "]", label)]
    );
  }
  if (typeof value === "object") {
    return normalizeJsonObject(value as object, path, label);
  }

  throw new Error("Unsupported " + label + " at " + path + ": expected JSON-like value");
}

const fabricReservedProps = new Set(["children", "key", "ref", "style", "queries"]);

function normalizeFabricProps(type: string, props: HostProps | null | undefined): JsonObject {
  const result: JsonObject = {};
  for (const [key, value] of Object.entries(props ?? {})) {
    if (fabricReservedProps.has(key) || value === undefined || typeof value === "function") {
      continue;
    }
    result[key] = normalizeJsonValue(value, "props." + key, type + " prop");
  }
  return result;
}

function normalizeFabricStyle(type: string, props: HostProps | null | undefined): JsonValue {
  return normalizeJsonValue(props?.style ?? {}, "style", type + " style");
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

export function normalizeFabricBasePayload(type: string, props: HostProps | null | undefined, hidden = false): RasterShadowNodePayload {
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
    [TEXT_EVENT_COUNT_PROP]: readTextControlEventCount(surfaceId, nodeTag),
  };
  return payload;
}

export function normalizeFabricTextPayload(text: unknown, hidden = false): RasterShadowNodePayload {
  return {
    props: {},
    style: {},
    text: String(text),
    hidden,
  };
}

export function hasFabricHandlers(props: HostProps | null | undefined): boolean {
  return collectFabricHandlers(props).length > 0;
}

export function stableJsonStringify(value: JsonValue | undefined): string {
  if (value == null || typeof value !== "object") {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return "[" + value.map((child) => stableJsonStringify(child)).join(",") + "]";
  }

  const entries = Object.entries(value).sort(([left], [right]) => left.localeCompare(right));
  return "{" + entries.map(([key, child]) => JSON.stringify(key) + ":" + stableJsonStringify(child)).join(",") + "}";
}

function fabricBasePayloadKey(type: string, props: HostProps | null | undefined, hidden: boolean): string {
  const payload = normalizeFabricBasePayload(type, props, hidden);
  return stableJsonStringify(payload as JsonObject);
}

export function fabricHandlerShapeKey(props: HostProps | null | undefined): string {
  const handlers = collectFabricHandlers(props)
    .map((handler) => ({
      kind: handler.kind,
      name: handler.name,
      eventOrQueryType: handler.eventOrQueryType,
    }))
    .sort((left, right) => {
      const leftKey = left.kind + ":" + left.name + ":" + (left.eventOrQueryType ?? "");
      const rightKey = right.kind + ":" + right.name + ":" + (right.eventOrQueryType ?? "");
      return leftKey.localeCompare(rightKey);
    });
  return stableJsonStringify(handlers as JsonValue);
}

export function shouldDirtyFabricHostProps(
  type: string,
  oldProps: HostProps | null | undefined,
  newProps: HostProps | null | undefined,
  oldHidden: boolean,
  newHidden: boolean,
  oldHandlerShapeKey: string
): boolean {
  return (
    oldHidden !== newHidden ||
    fabricBasePayloadKey(type, oldProps, oldHidden) !== fabricBasePayloadKey(type, newProps, newHidden) ||
    oldHandlerShapeKey !== fabricHandlerShapeKey(newProps)
  );
}

export function attachFabricHandlerBindings(
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
      events.push({ property: handler.name, event_type: handler.eventOrQueryType, handler_slot_id: slotId });
    } else {
      queries.push({ property: handler.name, query_type: handler.eventOrQueryType, handler_slot_id: slotId });
    }
  }

  payload.event_bindings = events;
  payload.query_bindings = queries;
  return payload;
}

export function normalizeFabricPayloadForNode(
  surfaceId: RasterSurfaceId,
  nodeTag: RasterNodeTag,
  type: string,
  props: HostProps | null | undefined,
  hidden: boolean,
  binding: RasterNativeBinding
): RasterShadowNodePayload {
  return attachTextControlEventCount(
    attachFabricHandlerBindings(normalizeFabricBasePayload(type, props, hidden), surfaceId, nodeTag, props, binding),
    surfaceId,
    nodeTag,
    type,
    props
  );
}

