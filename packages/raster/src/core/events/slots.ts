import type { RasterNativeBinding, RasterNativeJsFunctionRef, RasterNodeTag, RasterSurfaceId } from "../types/index.js";
import { getRasterNativeBinding } from "../runtime/index.js";
import type { HandlerId, HandlerKind, HandlerSlotKey, RasterHandler } from "./state.js";
import { handlers, handlerSlots } from "./state.js";

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

export function handlerSlotKey(surfaceId: RasterSurfaceId, nodeTag: RasterNodeTag, kind: HandlerKind, name: string): HandlerSlotKey {
  return surfaceId + ":" + nodeTag + ":" + kind + ":" + name;
}

function handlerSlotKeyForDescriptor(descriptor: HandlerSlotDescriptor): HandlerSlotKey {
  return handlerSlotKey(descriptor.surfaceId, descriptor.nodeTag, descriptor.kind, descriptor.name);
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
  const prefix = surfaceId + ":" + nodeTag + ":";
  for (const [key, id] of Array.from(handlerSlots)) {
    if (key.startsWith(prefix)) {
      handlerSlots.delete(key);
      handlers.delete(id);
    }
  }
  binding?.dropHandlerSlotsForNode(surfaceId, nodeTag);
}

export function collectFabricHandlers(props: Record<string, unknown> | null | undefined): Array<{
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
          throw new Error('Fabric query "' + queryName + '" must be a function');
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

