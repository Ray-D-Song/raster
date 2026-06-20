import "../events/install.js";

import type { RasterNativeBinding } from "../types/index.js";
import { getRasterNativeBinding } from "../runtime/index.js";
import type { RasterFabricContainer } from "../runtime/index.js";

export { createFabricContainer, getRasterNativeBinding, type RasterFabricContainer } from "../runtime/index.js";
export { readFabricChildHandles } from "./state.js";
export {
  appendInitialFabricChild,
  appendFabricChild,
  appendFabricChildToContainer,
  insertFabricChildBefore,
  insertFabricChildInContainerBefore,
  removeFabricChild,
  removeFabricChildFromContainer,
} from "./children.js";
export {
  cloneFabricHostNode,
  cloneFabricTextNode,
  createFabricHostNode,
  createFabricTextNode,
  deleteFabricNode,
  updateFabricHostNode,
  updateFabricTextNode,
} from "./node.js";
export {
  appendFabricChildToSet,
  commitFabricChildSet,
  createFabricChildSet,
  finalizeFabricChildSet,
  readRasterFabricMaterializeDiagnostics,
  resetRasterFabricMaterializeDiagnostics,
  type RasterFabricMaterializeDiagnostics,
} from "./materialize.js";

export function prepareFabricCommit(
  container: RasterFabricContainer,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.prepareForCommit(container.surfaceId);
}

export function resetFabricCommit(
  container: RasterFabricContainer,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.resetAfterCommit(container.surfaceId);
}

export function clearFabricSurface(
  container: RasterFabricContainer,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.clearSurface?.(container.surfaceId);
}
