import type { RasterNativeBinding, RasterNativeNode } from "../types/index.js";
import { getRasterNativeBinding } from "../runtime/index.js";
import { fabricNodeChildHandles, fabricNodeShadows, readFabricChildHandles } from "./state.js";

export function appendInitialFabricChild(
  parent: RasterNativeNode,
  child: RasterNativeNode,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.appendInitialChild(parent.handle, child.handle);
  const shadow = fabricNodeShadows.get(parent);
  if (shadow != null) {
    const index = shadow.childNodes.length;
    shadow.childNodes = [...shadow.childNodes, child];
    shadow.childrenDirty = true;
    shadow.dirtyChildIndexes.add(index);
    return;
  }

  const nextChildren = [...readFabricChildHandles(parent), child.handle];
  fabricNodeChildHandles.set(parent, nextChildren);
}

export function appendFabricChild(parent: RasterNativeNode, child: RasterNativeNode, binding: RasterNativeBinding = getRasterNativeBinding()): void {
  binding.appendChild(parent.handle, child.handle);
}

export function appendFabricChildToContainer(_container: { surfaceId: number }, child: RasterNativeNode, binding: RasterNativeBinding = getRasterNativeBinding()): void {
  binding.appendChildToContainer(_container.surfaceId, child.handle);
}

export function insertFabricChildBefore(
  parent: RasterNativeNode,
  child: RasterNativeNode,
  before: RasterNativeNode,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.insertBefore(parent.handle, child.handle, before.handle);
}

export function insertFabricChildInContainerBefore(
  container: { surfaceId: number },
  child: RasterNativeNode,
  before: RasterNativeNode,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.insertInContainerBefore(container.surfaceId, child.handle, before.handle);
}

export function removeFabricChild(parent: RasterNativeNode, child: RasterNativeNode, binding: RasterNativeBinding = getRasterNativeBinding()): void {
  binding.removeChild(parent.handle, child.handle);
}

export function removeFabricChildFromContainer(container: { surfaceId: number }, child: RasterNativeNode, binding: RasterNativeBinding = getRasterNativeBinding()): void {
  binding.removeChildFromContainer(container.surfaceId, child.handle);
}

