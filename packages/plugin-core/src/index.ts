import { getRasterBridge } from "raster-js/bridge";
import { addRasterRuntimeEventListener } from "raster-js/core";

export type PermissionState = "prompt" | "granted" | "denied" | "limited";

export interface PermissionStatus {
  camera?: PermissionState;
  photos?: PermissionState;
  [key: string]: PermissionState | undefined;
}

export class PluginError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "PluginError";
    this.code = code;
  }
}

export interface PluginListenerHandle {
  remove: () => void;
}

export function registerPlugin<P extends object>(name: string): P {
  return new Proxy({} as P, {
    get(_target, method: string | symbol) {
      if (typeof method !== "string") {
        return undefined;
      }
      return (args?: unknown) =>
        getRasterBridge().call("host.plugin", "invoke", {
          plugin: name,
          method,
          args: args ?? null,
        });
    },
  }) as P;
}

export function addPluginListener(
  plugin: string,
  event: string,
  handler: (data: unknown) => void
): PluginListenerHandle {
  const remove = addRasterRuntimeEventListener(`plugin:${plugin}:${event}`, handler);
  return { remove };
}

export function parsePluginError(error: unknown): PluginError {
  if (error instanceof PluginError) {
    return error;
  }
  if (error instanceof Error) {
    try {
      const parsed = JSON.parse(error.message) as { code?: string; message?: string };
      if (typeof parsed.code === "string") {
        return new PluginError(parsed.code, parsed.message ?? error.message);
      }
    } catch {
      // fall through
    }
    return new PluginError("PLUGIN_ERROR", error.message);
  }
  return new PluginError("PLUGIN_ERROR", String(error));
}