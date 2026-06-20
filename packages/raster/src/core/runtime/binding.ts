import type { RasterNativeBinding } from "../types/index.js";
import { rasterGlobal } from "./global.js";

export function getRasterNativeBinding(): RasterNativeBinding {
  const binding = rasterGlobal.__rasterNative;
  if (binding == null) {
    throw new Error("Raster Fabric renderer requires globalThis.__rasterNative");
  }
  return binding;
}

