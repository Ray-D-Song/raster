import type { JsonValue } from "../core/types/json.js";

export type BridgeMessageKind = "call" | "reply" | "event";

export interface BridgeDispatchMessage {
  kind: BridgeMessageKind;
  id?: number;
  ok?: boolean;
  error?: string;
  channel?: string;
  method?: string;
  name?: string;
  payload?: JsonValue;
}

export type BridgePayload = JsonValue | ArrayBuffer;

export interface BridgeCallOptions {
  bytes?: ArrayBuffer;
}