import type { HandlerRecord } from "./state.js";
import { textControlEventCounts } from "./state.js";
import type { RasterNodeTag, RasterSurfaceId } from "../types/index.js";

export const TEXT_EVENT_COUNT_PROP = "__rasterTextEventCount";

export function textControlKey(surfaceId: RasterSurfaceId, nodeTag: RasterNodeTag): string {
  return surfaceId + ":" + nodeTag;
}

export function isTextControlType(type: string): boolean {
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

export function recordTextControlEventCount(record: HandlerRecord, payload: unknown): void {
  if (record.name !== "onChange" && record.name !== "onChangeText") {
    return;
  }
  const textPayload = normalizeTextEventPayload(payload);
  if (textPayload == null) {
    return;
  }
  textControlEventCounts.set(textControlKey(record.surfaceId, record.nodeTag), textPayload.eventCount);
}

export function userPayloadForHandler(record: HandlerRecord, payload: unknown): unknown {
  if (record.name !== "onChangeText") {
    return payload;
  }
  return normalizeTextEventPayload(payload)?.value ?? payload;
}

export function readTextControlEventCount(surfaceId: RasterSurfaceId, nodeTag: RasterNodeTag): number {
  return textControlEventCounts.get(textControlKey(surfaceId, nodeTag)) ?? 0;
}

