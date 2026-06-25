import { resetRasterHandlerRegistry } from "../events/state.js";
import { resetRasterRuntimeEventListeners } from "../runtime-events.js";
import type { RasterRuntimeGlobal } from "./global.js";

/** Clears handler/event state while keeping installed runtime globals alive. */
export function resetRasterRuntimeHandlerState(): void {
  resetRasterRuntimeEventListeners();
  resetRasterHandlerRegistry();
}

export function resetRasterRuntimeGlobals(): void {
  resetRasterRuntimeHandlerState();

  const runtimeGlobal = globalThis as RasterRuntimeGlobal;
  delete runtimeGlobal.__rasterHandlerRegistry;
  delete runtimeGlobal.__rasterInvokeHandler;
  delete runtimeGlobal.__rasterInvokeHandlerJson;
  delete runtimeGlobal.__rasterInvokeHandlersJson;
  delete runtimeGlobal.__rasterInvokeQuery;
  delete runtimeGlobal.__rasterInvokeQueryJson;
  delete runtimeGlobal.__rasterBridgeDispatch;
  delete runtimeGlobal.__rasterReadMaterializeDiagnostics;
  delete runtimeGlobal.__rasterResetMaterializeDiagnostics;
}