import { rasterGlobal, type RasterEventMetadata } from "../runtime/index.js";
import type { HandlerId } from "./state.js";
import { handlers } from "./state.js";
import { recordTextControlEventCount, userPayloadForHandler } from "./text-control.js";

function createEventMetadata(eventType: string | null): RasterEventMetadata | null {
  if (eventType == null) {
    return null;
  }
  return { type: eventType, timeStamp: Date.now() };
}

export function installRasterEventHandlers(): void {
  rasterGlobal.__rasterInvokeHandler = function (id: HandlerId, payload: unknown): unknown {
    const record = handlers.get(id);
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
  };

  rasterGlobal.__rasterInvokeHandlerJson = function (id: HandlerId, payloadJson: string): unknown {
    return rasterGlobal.__rasterInvokeHandler?.(id, JSON.parse(payloadJson));
  };

  rasterGlobal.__rasterInvokeHandlersJson = function (callsJson: string): unknown[] {
    const calls = JSON.parse(callsJson) as Array<{ id: HandlerId; payload: unknown }>;
    const resolvedCalls = calls.map((call) => ({ call, record: handlers.get(call.id) }));
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
          ? rasterGlobal.__rasterRunEvent(record.handler, userPayload, createEventMetadata(record.eventType))
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

  rasterGlobal.__rasterInvokeQueryJson = function (id: HandlerId, payloadJson: string): string {
    const result = rasterGlobal.__rasterInvokeQuery?.(id, JSON.parse(payloadJson));
    return JSON.stringify(result ?? null);
  };
}

