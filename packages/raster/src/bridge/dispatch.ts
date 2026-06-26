import type { JsonValue } from "../core/types/json.js";
import { rasterGlobal } from "../core/runtime/global.js";
import { handlers } from "../core/events/state.js";
import { recordTextControlEventCount, userPayloadForHandler } from "../core/events/text-control.js";
import type { HandlerId } from "../core/events/state.js";
import type { RasterEventMetadata } from "../core/runtime/global.js";
import { dispatchRasterRuntimeEvent } from "../core/runtime-events.js";
import type { BridgeDispatchMessage } from "./types.js";

function createEventMetadata(eventType: string | null): RasterEventMetadata | null {
  if (eventType == null) {
    return null;
  }
  return { type: eventType, timeStamp: Date.now() };
}

function invokeHandler(handlerId: HandlerId, payload: unknown): unknown {
  const record = handlers.get(handlerId);
  if (!record) {
    return undefined;
  }
  recordTextControlEventCount(record, payload);
  const userPayload = userPayloadForHandler(record, payload);
  const result =
    typeof rasterGlobal.__rasterRunEvent === "function"
      ? rasterGlobal.__rasterRunEvent(record.handler, userPayload, createEventMetadata(record.eventType))
      : record.handler(userPayload);
  if (typeof rasterGlobal.__rasterFlushSyncWork === "function") {
    rasterGlobal.__rasterFlushSyncWork();
  }
  return result;
}

function handleBridgeEvent(message: BridgeDispatchMessage): void {
  if (message.channel === "host.event" && message.name === "invoke") {
    const payload = message.payload;
    if (payload == null || typeof payload !== "object" || Array.isArray(payload)) {
      return;
    }
    const handlerId = (payload as Record<string, unknown>).handlerId;
    const data = (payload as Record<string, unknown>).payload;
    if (typeof handlerId === "number") {
      invokeHandler(handlerId, data);
    }
    return;
  }

  if (message.channel === "runtime.lifecycle" && message.name != null) {
    dispatchRasterRuntimeEvent(message.name, message.payload ?? null);
    if (typeof rasterGlobal.__rasterFlushSyncWork === "function") {
      rasterGlobal.__rasterFlushSyncWork();
    }
    return;
  }

  if (message.channel === "plugin.event" && message.name != null) {
    const payload = message.payload;
    const eventName =
      payload != null && typeof payload === "object" && !Array.isArray(payload)
        ? (payload as Record<string, unknown>).event
        : null;
    const data =
      payload != null && typeof payload === "object" && !Array.isArray(payload)
        ? (payload as Record<string, unknown>).data ?? null
        : null;
    if (typeof eventName === "string") {
      dispatchRasterRuntimeEvent(`plugin:${message.name}:${eventName}`, data);
      if (typeof rasterGlobal.__rasterFlushSyncWork === "function") {
        rasterGlobal.__rasterFlushSyncWork();
      }
    }
  }
}

const pendingReplies = new Map<number, { resolve: (value: JsonValue) => void; reject: (error: Error) => void }>();

export function installBridgeDispatch(): void {
  rasterGlobal.__rasterBridgeDispatch = (message) => {
    const typed = message as BridgeDispatchMessage;
    if (typed.kind === "event") {
      handleBridgeEvent(typed);
      return;
    }
    if (typed.kind === "reply" && typeof typed.id === "number") {
      const pending = pendingReplies.get(typed.id);
      if (pending == null) {
        return;
      }
      pendingReplies.delete(typed.id);
      if (typed.ok === false) {
        pending.reject(new Error(typed.error ?? "bridge call failed"));
        return;
      }
      pending.resolve(typed.payload ?? null);
    }
  };
}

export function registerBridgeReply(id: number, resolve: (value: JsonValue) => void, reject: (error: Error) => void): void {
  pendingReplies.set(id, { resolve, reject });
}