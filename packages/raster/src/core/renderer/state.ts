import type { RasterNativeChildSet, RasterNativeNode, RasterNativeNodeHandle } from "../types/index.js";
import { RASTER_NATIVE_NODE_TYPE } from "../runtime/index.js";

type HostProps = Record<string, unknown>;

export interface FabricShadowState {
  type: string | null;
  props: HostProps | null | undefined;
  key: string | null;
  text: string | null;
  hidden: boolean;
  childNodes: RasterNativeNode[];
  childHandles: RasterNativeNodeHandle[];
  handlerShapeKey: string;
  handlersDirty: boolean;
  propsDirty: boolean;
  childrenDirty: boolean;
  dirtyChildIndexes: Set<number>;
  committed: boolean;
}

export const fabricNodeChildHandles = new WeakMap<RasterNativeNode, RasterNativeNodeHandle[]>();
export const fabricChildSetNodes = new WeakMap<RasterNativeChildSet, RasterNativeNode[]>();
export const fabricNodeShadows = new WeakMap<RasterNativeNode, FabricShadowState>();

export function nativeNodeDebug(name: string, key: string | null): RasterNativeNode["debug"] {
  return key == null ? { name } : { name, key };
}

export function createNativeNodeWrapper(
  kind: RasterNativeNode["kind"],
  handle: RasterNativeNodeHandle,
  debug: RasterNativeNode["debug"],
  childHandles: readonly RasterNativeNodeHandle[] = [],
  shadow?: Omit<FabricShadowState, "childHandles">
): RasterNativeNode {
  const node = {
    $$typeof: RASTER_NATIVE_NODE_TYPE,
    kind,
    tag: handle.node_tag,
    handle,
    debug,
  } as RasterNativeNode;
  fabricNodeChildHandles.set(node, [...childHandles]);
  if (shadow != null) {
    fabricNodeShadows.set(node, {
      ...shadow,
      childHandles: [...childHandles],
      dirtyChildIndexes: new Set(shadow.dirtyChildIndexes),
    });
  }
  return node;
}

export function readFabricChildHandles(instance: RasterNativeNode): RasterNativeNodeHandle[] {
  return [...(fabricNodeShadows.get(instance)?.childHandles ?? fabricNodeChildHandles.get(instance) ?? [])];
}

export function handlesAreEqual(a: RasterNativeNodeHandle, b: RasterNativeNodeHandle): boolean {
  return (
    a.surface_id === b.surface_id &&
    a.node_tag === b.node_tag &&
    a.revision_id === b.revision_id &&
    a.generation === b.generation
  );
}

