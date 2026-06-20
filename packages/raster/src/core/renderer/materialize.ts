import type {
  RasterNativeBinding,
  RasterNativeChildSet,
  RasterNativeMaterializeChildUpdate,
  RasterNativeMaterializeResult,
  RasterNativeMaterializeSpec,
  RasterNativeNode,
  RasterNativeNodeHandle,
  RasterSurfaceId,
} from "../types/index.js";
import { getRasterNativeBinding, RASTER_NATIVE_CHILD_SET_TYPE, type RasterFabricContainer } from "../runtime/index.js";
import {
  fabricChildSetNodes,
  fabricNodeChildHandles,
  fabricNodeShadows,
  handlesAreEqual,
} from "./state.js";
import {
  attachFabricHandlerBindings,
  fabricHandlerShapeKey,
  normalizeFabricPayloadForNode,
  normalizeFabricTextPayload,
} from "./payload.js";

export interface RasterFabricMaterializeDiagnostics {
  lastVisitedNodes: number;
  lastEmittedNodes: number;
  lastCommittedRoots: number;
  totalVisitedNodes: number;
  totalEmittedNodes: number;
  totalCommittedRoots: number;
  commitCount: number;
}

interface RasterFabricMaterializeCommitCounters {
  visitedNodes: number;
  emittedNodes: number;
  committedRoots: number;
}

const fabricMaterializeDiagnostics: RasterFabricMaterializeDiagnostics = {
  lastVisitedNodes: 0,
  lastEmittedNodes: 0,
  lastCommittedRoots: 0,
  totalVisitedNodes: 0,
  totalEmittedNodes: 0,
  totalCommittedRoots: 0,
  commitCount: 0,
};

type FabricMaterializeFrame = {
  spec: RasterNativeMaterializeSpec;
  node: RasterNativeNode;
  handleMayChange: boolean;
};

interface FabricMaterializeBuildContext {
  binding: RasterNativeBinding;
  counters: RasterFabricMaterializeCommitCounters;
  dirtyNodes: Set<RasterNativeNode>;
}

function recordFabricMaterializeCommit(counters: RasterFabricMaterializeCommitCounters): void {
  fabricMaterializeDiagnostics.lastVisitedNodes = counters.visitedNodes;
  fabricMaterializeDiagnostics.lastEmittedNodes = counters.emittedNodes;
  fabricMaterializeDiagnostics.lastCommittedRoots = counters.committedRoots;
  fabricMaterializeDiagnostics.totalVisitedNodes += counters.visitedNodes;
  fabricMaterializeDiagnostics.totalEmittedNodes += counters.emittedNodes;
  fabricMaterializeDiagnostics.totalCommittedRoots += counters.committedRoots;
  fabricMaterializeDiagnostics.commitCount += 1;
}

export function readRasterFabricMaterializeDiagnostics(): RasterFabricMaterializeDiagnostics {
  return { ...fabricMaterializeDiagnostics };
}

export function resetRasterFabricMaterializeDiagnostics(): void {
  fabricMaterializeDiagnostics.lastVisitedNodes = 0;
  fabricMaterializeDiagnostics.lastEmittedNodes = 0;
  fabricMaterializeDiagnostics.lastCommittedRoots = 0;
  fabricMaterializeDiagnostics.totalVisitedNodes = 0;
  fabricMaterializeDiagnostics.totalEmittedNodes = 0;
  fabricMaterializeDiagnostics.totalCommittedRoots = 0;
  fabricMaterializeDiagnostics.commitCount = 0;
}

export function fabricNodeNeedsMaterializeVisit(instance: RasterNativeNode): boolean {
  const shadow = fabricNodeShadows.get(instance);
  return shadow != null && (!shadow.committed || shadow.propsDirty || shadow.handlersDirty || fabricChildrenNeedMaterialize(shadow));
}

function fabricChildrenNeedMaterialize(shadow: NonNullable<ReturnType<typeof fabricNodeShadows.get>>): boolean {
  if (!shadow.childrenDirty && shadow.dirtyChildIndexes.size === 0) {
    return false;
  }
  if (shadow.childNodes.length !== shadow.childHandles.length) {
    return true;
  }

  return shadow.childNodes.some((child, index) => {
    const previousHandle = shadow.childHandles[index];
    return previousHandle == null || !handlesAreEqual(child.handle, previousHandle) || fabricNodeNeedsMaterializeVisit(child);
  });
}

function buildFabricShallowMaterializeFrame(instance: RasterNativeNode, context: FabricMaterializeBuildContext): FabricMaterializeFrame {
  context.counters.emittedNodes += 1;
  return { spec: { handle: instance.handle }, node: instance, handleMayChange: false };
}

function buildFabricMaterializeFrame(instance: RasterNativeNode, context: FabricMaterializeBuildContext): FabricMaterializeFrame {
  const frames = new WeakMap<RasterNativeNode, FabricMaterializeFrame>();
  const stack: Array<{ node: RasterNativeNode; visited: boolean }> = [{ node: instance, visited: false }];

  while (stack.length > 0) {
    const frame = stack.pop()!;
    if (!fabricNodeNeedsMaterializeVisit(frame.node)) {
      frames.set(frame.node, buildFabricShallowMaterializeFrame(frame.node, context));
      continue;
    }

    const shadow = fabricNodeShadows.get(frame.node);
    if (shadow == null) {
      frames.set(frame.node, buildFabricShallowMaterializeFrame(frame.node, context));
      continue;
    }

    if (!frame.visited) {
      stack.push({ node: frame.node, visited: true });
      if (frame.node.kind !== "text" && fabricChildrenNeedMaterialize(shadow)) {
        for (let index = shadow.childNodes.length - 1; index >= 0; index -= 1) {
          const child = shadow.childNodes[index]!;
          if (fabricNodeNeedsMaterializeVisit(child) && !frames.has(child)) {
            stack.push({ node: child, visited: false });
          }
        }
      }
      continue;
    }

    frames.set(frame.node, buildFabricMaterializeFrameFromChildren(frame.node, shadow, context, frames));
  }

  return frames.get(instance) ?? { spec: { handle: instance.handle }, node: instance, handleMayChange: false };
}

function buildFabricMaterializeFrameFromChildren(
  instance: RasterNativeNode,
  shadow: NonNullable<ReturnType<typeof fabricNodeShadows.get>>,
  context: FabricMaterializeBuildContext,
  frames: WeakMap<RasterNativeNode, FabricMaterializeFrame>
): FabricMaterializeFrame {
  context.counters.emittedNodes += 1;
  context.counters.visitedNodes += 1;
  context.dirtyNodes.add(instance);

  let spec: RasterNativeMaterializeSpec = { handle: instance.handle };
  let handleMayChange = false;

  if (instance.kind === "text") {
    if (!shadow.committed) {
      const nextText = shadow.text ?? "";
      spec = { kind: "text", text: nextText, payload: normalizeFabricTextPayload(nextText, shadow.hidden) };
      handleMayChange = true;
    } else if (shadow.propsDirty) {
      const nextText = shadow.text ?? "";
      spec = { handle: instance.handle, payload: normalizeFabricTextPayload(nextText, shadow.hidden) };
      handleMayChange = true;
    }
    return { spec, node: instance, handleMayChange };
  }

  const childrenNeedMaterialize = fabricChildrenNeedMaterialize(shadow);
  const childUpdates: RasterNativeMaterializeChildUpdate[] = [];
  const useFullChildren = childrenNeedMaterialize && shadow.childHandles.length === 0;
  const childFrames = childrenNeedMaterialize
    ? shadow.childNodes.map((child, index) => {
        if (!useFullChildren && !fabricNodeNeedsMaterializeVisit(child)) {
          return null;
        }
        const childFrame = frames.get(child) ?? buildFabricShallowMaterializeFrame(child, context);
        if (!useFullChildren) {
          childUpdates.push({ index, child: childFrame.spec });
        }
        return childFrame;
      })
    : [];
  const childHandleMayChange = childFrames.some((child) => child?.handleMayChange);
  const needsChildren = childrenNeedMaterialize || childHandleMayChange;
  const type = shadow.type ?? instance.debug?.name ?? "View";

  if (shadow.handlersDirty && !shadow.propsDirty) {
    attachFabricHandlerBindings({}, instance.handle.surface_id, instance.handle.node_tag, shadow.props, context.binding);
  }

  const payload = shadow.propsDirty
    ? normalizeFabricPayloadForNode(instance.handle.surface_id, instance.handle.node_tag, type, shadow.props, shadow.hidden, context.binding)
    : {};
  if (shadow.propsDirty || needsChildren) {
    spec = { handle: instance.handle, payload };
    if (needsChildren) {
      if (useFullChildren) {
        spec.children = childFrames.filter((child): child is FabricMaterializeFrame => child != null).map((child) => child.spec);
      } else {
        spec.childHandles = shadow.childNodes.map((child) => child.handle);
        if (childUpdates.length > 0) {
          spec.childUpdates = childUpdates;
        }
      }
    }
    handleMayChange = true;
  }

  return { spec, node: instance, handleMayChange };
}

function markFabricNodeMaterialized(instance: RasterNativeNode): void {
  const shadow = fabricNodeShadows.get(instance);
  if (shadow == null) {
    return;
  }
  shadow.handlersDirty = false;
  shadow.propsDirty = false;
  shadow.childrenDirty = false;
  shadow.dirtyChildIndexes.clear();
  shadow.committed = true;
  shadow.handlerShapeKey = fabricHandlerShapeKey(shadow.props);
  const nextChildHandles = shadow.childNodes.map((child) => child.handle);
  shadow.childHandles = nextChildHandles;
  fabricNodeChildHandles.set(instance, nextChildHandles);
}

function applyFabricMaterializeResult(instance: RasterNativeNode, result: RasterNativeMaterializeResult): void {
  instance.handle = result.handle;
  instance.tag = result.handle.node_tag;
  const shadow = fabricNodeShadows.get(instance);
  if (shadow == null) {
    return;
  }
  const childUpdates = result.childUpdates ?? [];
  for (const childUpdate of childUpdates) {
    const child = shadow.childNodes[childUpdate.index];
    if (child != null) {
      applyFabricMaterializeResult(child, childUpdate.child);
    }
  }
  for (let index = 0; index < result.children.length; index += 1) {
    const child = shadow.childNodes[index];
    const childResult = result.children[index];
    if (child != null && childResult != null) {
      applyFabricMaterializeResult(child, childResult);
    }
  }
  const nextChildHandles = shadow.childNodes.map((child) => child.handle);
  shadow.propsDirty = false;
  shadow.handlersDirty = false;
  shadow.childrenDirty = false;
  shadow.dirtyChildIndexes.clear();
  shadow.committed = true;
  shadow.handlerShapeKey = fabricHandlerShapeKey(shadow.props);
  shadow.childHandles = nextChildHandles;
  fabricNodeChildHandles.set(instance, nextChildHandles);
}

function commitFabricNodesToSurface(surfaceId: RasterSurfaceId, children: RasterNativeNode[], binding: RasterNativeBinding = getRasterNativeBinding()): void {
  const counters: RasterFabricMaterializeCommitCounters = { visitedNodes: 0, emittedNodes: 0, committedRoots: children.length };
  const context: FabricMaterializeBuildContext = { binding, counters, dirtyNodes: new Set() };
  const frames = children.map((child) => buildFabricMaterializeFrame(child, context));
  const results = binding.commitSurfaceTree(surfaceId, frames.map((frame) => frame.spec));
  for (let index = 0; index < results.length; index += 1) {
    const child = children[index];
    const result = results[index];
    if (child != null && result != null) {
      applyFabricMaterializeResult(child, result);
    }
  }
  for (const child of context.dirtyNodes) {
    markFabricNodeMaterialized(child);
  }
  recordFabricMaterializeCommit(counters);
}

function clearFabricChildSet(childSet: RasterNativeChildSet): RasterNativeNode[] {
  const children = fabricChildSetNodes.get(childSet) ?? [];
  fabricChildSetNodes.delete(childSet);
  return children;
}

function createSyntheticChildSetHandle(surfaceId: RasterSurfaceId): RasterNativeChildSet["handle"] {
  return { surface_id: surfaceId, child_set_id: 0, generation: 0 };
}

export function createFabricChildSet(container: RasterFabricContainer, binding: RasterNativeBinding = getRasterNativeBinding()): RasterNativeChildSet {
  void binding;
  const childSet = { $$typeof: RASTER_NATIVE_CHILD_SET_TYPE, surfaceId: container.surfaceId, handle: createSyntheticChildSetHandle(container.surfaceId) } as RasterNativeChildSet;
  fabricChildSetNodes.set(childSet, []);
  return childSet;
}

export function appendFabricChildToSet(childSet: RasterNativeChildSet, child: RasterNativeNode, binding: RasterNativeBinding = getRasterNativeBinding()): void {
  void binding;
  const children = fabricChildSetNodes.get(childSet);
  if (children == null) {
    throw new Error("Cannot append to an unknown Raster Fabric child set");
  }
  children.push(child);
}

export function finalizeFabricChildSet(childSet: RasterNativeChildSet, binding: RasterNativeBinding = getRasterNativeBinding()): void {
  void childSet;
  void binding;
}

export function commitFabricChildSet(container: RasterFabricContainer, childSet: RasterNativeChildSet, binding: RasterNativeBinding = getRasterNativeBinding()): void {
  commitFabricNodesToSurface(container.surfaceId, clearFabricChildSet(childSet), binding);
}
