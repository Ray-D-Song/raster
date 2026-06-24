import { resetRasterHandlerRegistry } from "../events/state.js";
import { resetRasterRuntimeEventListeners } from "../runtime-events.js";
import type { RasterRuntimeGlobal } from "./global.js";

export function resetRasterRuntimeGlobals(): void {
  resetRasterRuntimeEventListeners();
  resetRasterHandlerRegistry();

  const runtimeGlobal = globalThis as RasterRuntimeGlobal;
  delete runtimeGlobal.__rasterHandlerRegistry;
  delete runtimeGlobal.__rasterInvokeHandler;
  delete runtimeGlobal.__rasterInvokeHandlerJson;
  delete runtimeGlobal.__rasterInvokeHandlersJson;
  delete runtimeGlobal.__rasterInvokeQuery;
  delete runtimeGlobal.__rasterInvokeQueryJson;
  delete runtimeGlobal.__rasterReadMaterializeDiagnostics;
  delete runtimeGlobal.__rasterResetMaterializeDiagnostics;
}