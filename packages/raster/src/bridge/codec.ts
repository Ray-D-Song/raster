import type { JsonObject, JsonValue } from "../core/types/json.js";

export function encodeBridgeBytes(bytes: ArrayBuffer): JsonObject {
  const view = new Uint8Array(bytes);
  let binary = "";
  for (let index = 0; index < view.length; index += 1) {
    binary += String.fromCharCode(view[index] ?? 0);
  }
  return {
    __bridgeBytes: true,
    data: btoa(binary),
  };
}

export function normalizeBridgePayload(payload: unknown): JsonValue {
  if (payload == null) {
    return null;
  }
  if (typeof payload === "boolean" || typeof payload === "string" || typeof payload === "number") {
    return payload;
  }
  if (payload instanceof ArrayBuffer) {
    return encodeBridgeBytes(payload);
  }
  if (ArrayBuffer.isView(payload)) {
    const view = new Uint8Array(payload.buffer, payload.byteOffset, payload.byteLength);
    return encodeBridgeBytes(view.buffer.slice(view.byteOffset, view.byteOffset + view.byteLength) as ArrayBuffer);
  }
  if (Array.isArray(payload)) {
    return payload.map((item) => normalizeBridgePayload(item));
  }
  if (typeof payload === "object") {
    const object = payload as Record<string, unknown>;
    const result: JsonObject = {};
    for (const [key, value] of Object.entries(object)) {
      if (value === undefined) {
        continue;
      }
      result[key] = normalizeBridgePayload(value);
    }
    return result;
  }
  return null;
}