import type { JsonValue } from "../core/types/json.js";
import { normalizeBridgePayload } from "./codec.js";
import { installBridgeDispatch, registerBridgeReply } from "./dispatch.js";

type RasterBridgeBinding = {
  call?: (channel: string, method: string, payload: unknown) => number;
  post?: (channel: string, method: string, payload: unknown) => void;
};

let installed = false;

function bridgeBinding(): RasterBridgeBinding {
  return globalThis.__rasterBridge as RasterBridgeBinding;
}

function ensureInstalled(): void {
  if (installed) {
    return;
  }
  installBridgeDispatch();
  installed = true;
}

export function getRasterBridge() {
  ensureInstalled();
  return {
    call<T extends JsonValue = JsonValue>(channel: string, method: string, payload: unknown = null): Promise<T> {
      const binding = bridgeBinding();
      if (binding.call == null) {
        return Promise.reject(new Error("Raster bridge is not available"));
      }
      const normalized = normalizeBridgePayload(payload);
      const id = binding.call(channel, method, normalized);
      if (id === 0) {
        return Promise.resolve(null as T);
      }
      return new Promise<T>((resolve, reject) => {
        registerBridgeReply(id, (value) => resolve(value as T), reject);
      });
    },
    post(channel: string, method: string, payload: unknown = null): void {
      const binding = bridgeBinding();
      if (binding.post == null) {
        throw new Error("Raster bridge post is not available");
      }
      binding.post(channel, method, normalizeBridgePayload(payload));
    },
  };
}

declare global {
  // eslint-disable-next-line no-var
  var __rasterBridge: RasterBridgeBinding | undefined;
}

export { installBridgeDispatch } from "./dispatch.js";
export type { BridgeDispatchMessage } from "./types.js";