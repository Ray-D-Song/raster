import type { RasterNativeBinding, RasterNativeNode, RasterNativeNodeKind, RasterSurfaceId } from "../types/index.js";
import { getRasterNativeBinding } from "../runtime/index.js";
import { dropRasterHandlerSlotsForNode } from "../events/index.js";
import {
  createNativeNodeWrapper,
  fabricNodeShadows,
  nativeNodeDebug,
  readFabricChildHandles,
} from "./state.js";
import {
  fabricHandlerShapeKey,
  hasFabricHandlers,
  normalizeFabricBasePayload,
  normalizeFabricPayloadForNode,
  normalizeFabricTextPayload,
  shouldDirtyFabricHostProps,
} from "./payload.js";
import { fabricNodeNeedsMaterializeVisit } from "./materialize.js";

type HostProps = Record<string, unknown>;

function nativeKindForHostType(type: string): RasterNativeNodeKind {
  switch (type) {
    case "Input":
      return "input";
    case "Textarea":
      return "textarea";
    case "Label":
    case "Widget":
      return "widget";
    case "Slot":
      return "slot";
    case "ConfigProvider":
      return "config_provider";
    default:
      return "host";
  }
}

export function createFabricHostNode(
  surfaceId: RasterSurfaceId,
  type: string,
  props: HostProps | null | undefined,
  key: string | null = null,
  binding: RasterNativeBinding = getRasterNativeBinding()
): RasterNativeNode {
  const kind = nativeKindForHostType(type);
  const initialPayload = normalizeFabricBasePayload(type, props, false);
  const handle = binding.createNode(surfaceId, kind, type, key == null ? null : String(key), initialPayload);
  binding.updateNode(handle, normalizeFabricPayloadForNode(surfaceId, handle.node_tag, type, props, false, binding));

  return createNativeNodeWrapper(kind, handle, nativeNodeDebug(type, key), [], {
    type,
    props,
    key,
    text: null,
    hidden: false,
    childNodes: [],
    handlerShapeKey: "",
    handlersDirty: hasFabricHandlers(props),
    propsDirty: hasFabricHandlers(props),
    childrenDirty: false,
    dirtyChildIndexes: new Set(),
    committed: true,
  });
}

export function createFabricTextNode(surfaceId: RasterSurfaceId, text: unknown, binding: RasterNativeBinding = getRasterNativeBinding()): RasterNativeNode {
  const nextText = String(text);
  const handle = binding.createTextNode(surfaceId, nextText, normalizeFabricTextPayload(nextText));
  return createNativeNodeWrapper("text", handle, { name: "Text" }, [], {
    type: null,
    props: null,
    key: null,
    text: nextText,
    hidden: false,
    childNodes: [],
    handlerShapeKey: "",
    handlersDirty: false,
    propsDirty: false,
    childrenDirty: false,
    dirtyChildIndexes: new Set(),
    committed: true,
  });
}

export function updateFabricHostNode(
  instance: RasterNativeNode,
  type: string,
  props: HostProps | null | undefined,
  hidden = false,
  binding: RasterNativeBinding = getRasterNativeBinding()
): void {
  binding.updateNode(instance.handle, normalizeFabricPayloadForNode(instance.handle.surface_id, instance.handle.node_tag, type, props, hidden, binding));
}

export function updateFabricTextNode(instance: RasterNativeNode, text: unknown, binding: RasterNativeBinding = getRasterNativeBinding()): void {
  binding.updateTextNode(instance.handle, String(text));
}

export function cloneFabricHostNode(
  instance: RasterNativeNode,
  type: string,
  _oldProps: HostProps | null | undefined,
  newProps: HostProps | null | undefined,
  hidden = false,
  binding: RasterNativeBinding = getRasterNativeBinding(),
  keepChildren = true
): RasterNativeNode {
  void binding;
  const previousShadow = fabricNodeShadows.get(instance);
  const childNodes = keepChildren ? [...(previousShadow?.childNodes ?? [])] : [];
  const dirtyChildIndexes = new Set<number>();
  if (keepChildren) {
    childNodes.forEach((child, index) => {
      if (fabricNodeNeedsMaterializeVisit(child)) {
        dirtyChildIndexes.add(index);
      }
    });
  }
  const childHandles = readFabricChildHandles(instance);
  const previousHidden = previousShadow?.hidden ?? false;
  const previousHandlerShapeKey = previousShadow?.handlerShapeKey ?? fabricHandlerShapeKey(_oldProps);
  return createNativeNodeWrapper(instance.kind, instance.handle, nativeNodeDebug(type, instance.debug?.key ?? null), childHandles, {
    type,
    props: newProps,
    key: instance.debug?.key ?? null,
    text: null,
    hidden,
    childNodes,
    handlerShapeKey: previousHandlerShapeKey,
    handlersDirty: hasFabricHandlers(newProps),
    propsDirty: shouldDirtyFabricHostProps(type, _oldProps, newProps, previousHidden, hidden, previousHandlerShapeKey),
    childrenDirty: !keepChildren || dirtyChildIndexes.size > 0,
    dirtyChildIndexes,
    committed: previousShadow?.committed ?? true,
  });
}

export function cloneFabricTextNode(instance: RasterNativeNode, text: unknown, hidden = false, binding: RasterNativeBinding = getRasterNativeBinding()): RasterNativeNode {
  const nextText = String(text);
  void binding;
  const previousText = fabricNodeShadows.get(instance)?.text;
  const previousHidden = fabricNodeShadows.get(instance)?.hidden ?? false;
  const previousCommitted = fabricNodeShadows.get(instance)?.committed ?? true;
  return createNativeNodeWrapper("text", instance.handle, instance.debug ?? { name: "Text" }, [], {
    type: null,
    props: null,
    key: null,
    text: nextText,
    hidden,
    childNodes: [],
    handlerShapeKey: "",
    handlersDirty: false,
    propsDirty: previousText !== nextText || previousHidden !== hidden,
    childrenDirty: false,
    dirtyChildIndexes: new Set(),
    committed: previousCommitted,
  });
}

export function deleteFabricNode(instance: RasterNativeNode, binding: RasterNativeBinding = getRasterNativeBinding()): void {
  dropRasterHandlerSlotsForNode(instance.handle.surface_id, instance.handle.node_tag, binding);
  binding.deleteNode(instance.handle);
}

