import type { RasterNativeBinding } from "../types/index.js";
import type { HandlerId, RasterHandler, RasterHandlerRegistry } from "../events/state.js";
import type { RasterFabricMaterializeDiagnostics } from "../renderer/materialize.js";

export interface RasterEventMetadata {
  type: string | null;
  timeStamp: number;
}

export type RasterRuntimeGlobal = typeof globalThis & {
  __rasterNative?: RasterNativeBinding;
  __rasterFlushSyncWork?: () => void;
  __rasterInvokeHandler?: (id: HandlerId, payload: unknown) => unknown;
  __rasterInvokeHandlerJson?: (id: HandlerId, payloadJson: string) => unknown;
  __rasterInvokeHandlersJson?: (callsJson: string) => unknown[];
  __rasterInvokeQuery?: (id: HandlerId, payload: unknown) => unknown;
  __rasterInvokeQueryJson?: (id: HandlerId, payloadJson: string) => string;
  __rasterHandlerRegistry?: RasterHandlerRegistry;
  __rasterReadMaterializeDiagnostics?: () => RasterFabricMaterializeDiagnostics;
  __rasterResetMaterializeDiagnostics?: () => void;
  __rasterRunEvent?: (handler: RasterHandler, payload: unknown, event: RasterEventMetadata | null) => unknown;
};

export const rasterGlobal = globalThis as RasterRuntimeGlobal;
export const RASTER_NATIVE_NODE_TYPE = "raster.native-node" as const;
export const RASTER_NATIVE_CHILD_SET_TYPE = "raster.native-child-set" as const;
export const RASTER_NATIVE_CONTAINER_TYPE = "raster.native-container" as const;
