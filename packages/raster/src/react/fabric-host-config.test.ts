import type {
  RasterNativeBinding,
  RasterNativeChildSetHandle,
  RasterNativeJsFunctionRef,
  RasterNativeMaterializeResult,
  RasterNativeMaterializeSpec,
  RasterNativeNodeHandle,
  RasterRootOptions,
} from "../core/types.js";

type NativeCall = {
  name: string;
  args?: readonly unknown[];
  handlerSlotId?: number;
  jsFunctionRef?: RasterNativeJsFunctionRef;
};

type RasterTestGlobal = typeof globalThis & {
  __rasterNative?: RasterNativeBinding;
  __rasterInvokeHandler?: (id: number, payload: unknown) => unknown;
  __rasterRendererVersion?: string;
};

function expect(condition: unknown, message: string): asserts condition {
  if (!condition) {
    throw new Error(message);
  }
}

function createMockBinding(): RasterNativeBinding & { calls: NativeCall[] } {
  const calls: NativeCall[] = [];
  let nextNodeTag = 1;
  let nextRevisionId = 1;
  let nextChildSetId = 1;
  let nextHandlerSlotId = 100;

  const createHandle = (surfaceId: number, nodeTag = nextNodeTag++): RasterNativeNodeHandle => ({
    surface_id: surfaceId,
    node_tag: nodeTag,
    revision_id: nextRevisionId++,
    generation: 1,
  });

  return {
    calls,
    createSurface(options?: RasterRootOptions) {
      calls.push({ name: "createSurface", args: [options] });
      return 42;
    },
    createNode(surfaceId, kind, name, key, payload) {
      const handle = createHandle(surfaceId);
      calls.push({ name: "createNode", args: [surfaceId, kind, name, key, payload] });
      return handle;
    },
    createTextNode(surfaceId, text, payload) {
      const handle = createHandle(surfaceId);
      calls.push({ name: "createTextNode", args: [surfaceId, text, payload] });
      return handle;
    },
    appendInitialChild(parent, child) {
      calls.push({ name: "appendInitialChild", args: [parent, child] });
    },
    prepareForCommit(surfaceId) {
      calls.push({ name: "prepareForCommit", args: [surfaceId] });
    },
    resetAfterCommit(surfaceId) {
      calls.push({ name: "resetAfterCommit", args: [surfaceId] });
    },
    appendChild(parent, child) {
      calls.push({ name: "appendChild", args: [parent, child] });
    },
    appendChildToContainer(surfaceId, child) {
      calls.push({ name: "appendChildToContainer", args: [surfaceId, child] });
    },
    insertBefore(parent, child, before) {
      calls.push({ name: "insertBefore", args: [parent, child, before] });
    },
    insertInContainerBefore(surfaceId, child, before) {
      calls.push({ name: "insertInContainerBefore", args: [surfaceId, child, before] });
    },
    removeChild(parent, child) {
      calls.push({ name: "removeChild", args: [parent, child] });
    },
    removeChildFromContainer(surfaceId, child) {
      calls.push({ name: "removeChildFromContainer", args: [surfaceId, child] });
    },
    updateNode(handle, payload) {
      calls.push({ name: "updateNode", args: [handle, payload] });
    },
    updateTextNode(handle, text) {
      calls.push({ name: "updateTextNode", args: [handle, text] });
    },
    cloneNode(handle, payload) {
      const nextHandle = createHandle(handle.surface_id, handle.node_tag);
      calls.push({ name: "cloneNode", args: [handle, payload] });
      return nextHandle;
    },
    cloneNodeWithChildren(handle, payload, children) {
      const nextHandle = createHandle(handle.surface_id, handle.node_tag);
      calls.push({ name: "cloneNodeWithChildren", args: [handle, payload, children] });
      return nextHandle;
    },
    createChildSet(surfaceId) {
      const childSet = {
        surface_id: surfaceId,
        child_set_id: nextChildSetId++,
        generation: 1,
      };
      calls.push({ name: "createChildSet", args: [surfaceId] });
      return childSet;
    },
    appendChildToSet(childSet: RasterNativeChildSetHandle, childHandle: RasterNativeNodeHandle) {
      calls.push({ name: "appendChildToSet", args: [childSet, childHandle] });
    },
    finalizeChildSet(childSet) {
      calls.push({ name: "finalizeChildSet", args: [childSet] });
    },
    commitChildSet(surfaceId, childSet) {
      calls.push({ name: "commitChildSet", args: [surfaceId, childSet] });
    },
    commitSurfaceTree(surfaceId, roots: RasterNativeMaterializeSpec[]) {
      calls.push({ name: "commitSurfaceTree", args: [surfaceId, roots] });
      return roots.map((root) => ({
        handle: root.handle ?? createHandle(surfaceId),
        children: [],
      })) as RasterNativeMaterializeResult[];
    },
    deleteNode(handle) {
      calls.push({ name: "deleteNode", args: [handle] });
    },
    registerHandlerSlot(surfaceId, nodeTag, kind, property, eventOrQueryType) {
      const handlerSlotId = nextHandlerSlotId++;
      calls.push({
        name: "registerHandlerSlot",
        args: [surfaceId, nodeTag, kind, property, eventOrQueryType],
        handlerSlotId,
      });
      return handlerSlotId;
    },
    updateHandlerSlot(handlerSlotId, jsFunctionRef) {
      calls.push({ name: "updateHandlerSlot", handlerSlotId, jsFunctionRef });
    },
    dropHandlerSlotsForNode(surfaceId, nodeTag) {
      calls.push({ name: "dropHandlerSlotsForNode", args: [surfaceId, nodeTag] });
    },
  };
}

function callNames(calls: NativeCall[]): string[] {
  return calls.map((call) => call.name);
}

const rasterGlobal = globalThis as RasterTestGlobal;
rasterGlobal.__rasterRendererVersion = "test";

const { __rasterFabricHostConfigInternals, createRoot } = await import("./index.js");
const { jsx } = await import("react/jsx-runtime");

function mutationHostConfigCommitsDomLikeOperations(): void {
  const binding = createMockBinding();
  rasterGlobal.__rasterNative = binding;

  try {
    const container = __rasterFabricHostConfigInternals.createContainer({ width: 320 });
    const host = __rasterFabricHostConfigInternals.createInstance(
      "View",
      { style: { width: 10 } },
      container,
      {},
      { key: "stable" }
    );
    const text = __rasterFabricHostConfigInternals.createTextInstance("child", container);
    __rasterFabricHostConfigInternals.appendInitialChild(host, text);

    const names = callNames(binding.calls);
    expect(names.includes("createNode"), "createInstance should create a retained host handle");
    expect(names.includes("createTextNode"), "createTextInstance should create a retained text handle");
    expect(names.includes("appendInitialChild"), "appendInitialChild should attach render-phase children locally");
    expect(!names.includes("cloneNode"), "mutation mode should not clone native nodes");
    expect(!names.includes("commitSurfaceTree"), "mutation mode should not materialize a full surface tree");
  } finally {
    delete rasterGlobal.__rasterNative;
  }
}

mutationHostConfigCommitsDomLikeOperations();

function defaultCreateRootUsesCommitBatchBoundary(): void {
  const binding = createMockBinding();
  rasterGlobal.__rasterNative = binding;

  try {
    const root = createRoot({ width: 800, height: 600 });
    root.render(
      jsx("View", {
        id: "root",
        children: jsx("View", {
          id: "child",
          children: "leaf",
        }),
      })
    );

    const names = callNames(binding.calls);
    expect(names.includes("prepareForCommit"), "React commit should open a host mutation batch");
    expect(names.includes("appendChildToContainer"), "React commit should append the root child to the surface");
    expect(names.includes("resetAfterCommit"), "React commit should flush the host mutation batch");
    expect(!names.includes("createChildSet"), "mutation mode should not create native child sets");
    expect(!names.includes("commitSurfaceTree"), "mutation mode should not commit full surface trees");
  } finally {
    delete rasterGlobal.__rasterNative;
  }
}

defaultCreateRootUsesCommitBatchBoundary();

function handlerUpdateReusesStableSlot(): void {
  const binding = createMockBinding();
  rasterGlobal.__rasterNative = binding;

  try {
    const firstHandler = () => "first";
    const nextHandler = () => "next";
    const renderTree = (handler: () => string) =>
      jsx("View", {
        id: "root",
        children: jsx("View", {
          onClick: handler,
          label: "Run",
        }),
      });

    const root = createRoot({ width: 800, height: 600 });
    root.render(renderTree(firstHandler));
    const handlerSlotId = binding.calls.find((call) => call.name === "registerHandlerSlot")
      ?.handlerSlotId;
    expect(handlerSlotId != null, "initial render should register an event handler slot");
    binding.calls.length = 0;

    root.render(renderTree(nextHandler));
    expect(
      rasterGlobal.__rasterInvokeHandler?.(handlerSlotId, {}) === "next",
      "handler-only updates should refresh the existing JS handler slot"
    );
    expect(
      !binding.calls.some((call) => call.name === "registerHandlerSlot"),
      "handler-only updates should reuse the existing native handler slot id"
    );
  } finally {
    delete rasterGlobal.__rasterNative;
  }
}

handlerUpdateReusesStableSlot();
