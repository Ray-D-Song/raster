import type { RasterRootOptions, RasterSurfaceId } from "../types/index.js";
import { getRasterNativeBinding } from "./binding.js";
import { RASTER_NATIVE_CONTAINER_TYPE } from "./global.js";
import type { RasterNativeBinding } from "../types/index.js";

export interface RasterFabricContainer {
  $$typeof: "raster.native-container";
  surfaceId: RasterSurfaceId;
  parent: null;
}

export function createFabricContainer(
  options?: RasterRootOptions,
  binding: RasterNativeBinding = getRasterNativeBinding()
): RasterFabricContainer {
  return {
    $$typeof: RASTER_NATIVE_CONTAINER_TYPE,
    surfaceId: binding.createSurface(options ?? {}),
    parent: null,
  };
}

