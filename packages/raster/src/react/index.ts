import { createContext, type ReactElement } from "react";
import Reconciler from "react-reconciler";
import type ReactReconciler = require("react-reconciler");
import {
  ConcurrentRoot,
  DefaultEventPriority,
  NoEventPriority,
} from "react-reconciler/constants.js";

import {
  appendInitialFabricChild,
  createFabricChildSet,
  createFabricContainer,
  createFabricHostNode,
  createFabricTextNode,
  appendFabricChild,
  appendFabricChildToContainer,
  deleteFabricNode,
  getRasterNativeBinding,
  insertFabricChildBefore,
  insertFabricChildInContainerBefore,
  prepareFabricCommit,
  removeFabricChild,
  removeFabricChildFromContainer,
  resetFabricCommit,
  clearFabricSurface,
  updateFabricHostNode,
  updateFabricTextNode,
  type RasterFabricContainer,
} from "../core/raster-core.js";
import type {
  RasterEventHandler,
  RasterNativeChildSet,
  RasterNativeNode,
  RasterRoot,
  RasterRootOptions,
} from "../core/types.js";

export {
  readRasterFabricMaterializeDiagnostics,
  resetRasterFabricMaterializeDiagnostics,
  type RasterFabricMaterializeDiagnostics,
} from "../core/raster-core.js";

type HostType = string;
type HostProps = Record<string, unknown>;
type HostContext = object;
type SuspenseInstance = never;
type HydratableInstance = never;
type TimeoutHandle = ReturnType<typeof setTimeout>;
type NoTimeout = -1;
type TransitionStatus = null;
type SuspendedState = null;
type EventPriority = number;
type FabricContainerNode = RasterFabricContainer;
type FabricHostInstance = RasterNativeNode;
type FabricTextInstance = RasterNativeNode;
type FabricChildSet = RasterNativeChildSet;

type ReactCurrentHostConfigExtensions = {
  rendererPackageName: string;
  rendererVersion: string;
  extraDevToolsConfig: null;
  getCurrentEventPriority(): EventPriority;
  maySuspendCommitOnUpdate(type: HostType, oldProps: HostProps, newProps: HostProps): boolean;
  maySuspendCommitInSyncRender(type: HostType, props: HostProps): boolean;
  suspendInstance(
    stateOrType?: SuspendedState | HostType,
    instanceOrProps?: unknown,
    type?: HostType,
    props?: HostProps
  ): void;
  suspendOnActiveViewTransition(state: SuspendedState, container: unknown): void;
  waitForCommitToBeReady(state?: SuspendedState, timeoutOffset?: number): null;
  getSuspendedCommitReason(state: SuspendedState, rootContainer: unknown): null;
  bindToConsole(methodName: string, args: unknown[], environmentName?: string): () => unknown;
};

type RasterErrorInfo = {
  componentStack?: string | null;
  errorBoundary?: unknown;
};

type FabricPublicInstance = FabricHostInstance | FabricTextInstance;
type FabricFormInstance = FabricHostInstance;
type FabricHostConfigBase = ReactReconciler.HostConfig<
  HostType,
  HostProps,
  FabricContainerNode,
  FabricHostInstance,
  FabricTextInstance,
  SuspenseInstance,
  HydratableInstance,
  FabricFormInstance,
  FabricPublicInstance,
  HostContext,
  FabricChildSet,
  TimeoutHandle,
  NoTimeout,
  TransitionStatus
>;
type FabricReconciler = Omit<
  ReactReconciler.Reconciler<
    FabricContainerNode,
    FabricHostInstance,
    FabricTextInstance,
    SuspenseInstance,
    FabricFormInstance,
    FabricPublicInstance
  >,
  "discreteUpdates"
> & {
  discreteUpdates<TPayload, TResult>(
    handler: (payload: TPayload) => TResult,
    payload: TPayload
  ): TResult;
  defaultOnUncaughtError(error: Error, errorInfo: RasterErrorInfo): void;
  defaultOnCaughtError(error: Error, errorInfo: RasterErrorInfo): void;
  defaultOnRecoverableError(error: Error, errorInfo: RasterErrorInfo): void;
  flushPassiveEffects?(): boolean;
};

type FabricContainer = ReturnType<FabricReconciler["createContainer"]>;

type RasterEventMetadata = {
  type: string | null;
  timeStamp: number;
};

type RasterRuntimeGlobal = typeof globalThis & {
  __rasterDevReload?: boolean;
  __rasterDevRoot?: RasterDevRoot;
  __rasterPrepareDevReload?: () => void;
  __rasterFlushSyncWork?: () => void;
  __rasterRunEvent?: (
    handler: RasterEventHandler,
    payload: unknown,
    event: RasterEventMetadata | null
  ) => unknown;
  __rasterRendererVersion?: string;
};

type RasterDevRoot = RasterRoot & {
  __rasterRunEvent: (
    handler: RasterEventHandler,
    payload: unknown,
    event: RasterEventMetadata | null
  ) => unknown;
  __rasterFlushSyncWork: () => void;
};

const rasterGlobal = globalThis as RasterRuntimeGlobal;
const rasterEventGlobal = globalThis as unknown as { event?: RasterEventMetadata };
const NotPendingTransition: TransitionStatus = null;
const HostTransitionContext = createContext<TransitionStatus>(
  NotPendingTransition
) as unknown as ReactReconciler.ReactContext<TransitionStatus>;
const rootHostContext = {};
const NO_EVENT_TIMESTAMP = -1.1;
const NOOP_DEFAULT_TRANSITION_INDICATOR = () => {};
const noFormatConsoleMethods = new Set(["dir", "dirxml", "groupEnd", "table"]);
let currentUpdatePriority = NoEventPriority;
let currentRasterEvent: RasterEventMetadata | null = null;
let schedulerEvent: RasterEventMetadata | null = null;
let currentFabricSurfaceId: number | null = null;

function rendererVersion(): string {
  const version = rasterGlobal.__rasterRendererVersion;
  if (typeof version !== "string" || version.length === 0) {
    throw new Error("react-raster requires __rasterRendererVersion to be installed before import");
  }
  return version;
}

function normalizeEventMetadata(event: RasterEventMetadata | null): RasterEventMetadata | null {
  if (event == null) {
    return null;
  }

  const timeStamp = Number(event.timeStamp);
  return {
    type: event.type == null ? null : String(event.type),
    timeStamp: Number.isFinite(timeStamp) ? timeStamp : Date.now(),
  };
}

function bindConsoleMethod(methodName: string, args: unknown[], environmentName = "unknown"): () => unknown {
  const method = (console as unknown as Record<string, unknown>)[methodName];
  const consoleMethod = typeof method === "function" ? method : console.log;
  const bind = Function.prototype.bind;

  if (noFormatConsoleMethods.has(methodName)) {
    return bind.apply(consoleMethod, [console, ...args]) as () => unknown;
  }

  const badgeFormat = "[%s]";
  const badgeLabel = ` ${environmentName} `;
  const nextArgs = args.slice();
  const offset = methodName === "assert" ? 1 : 0;

  if (typeof nextArgs[offset] === "string") {
    nextArgs.splice(offset, 1, `${badgeFormat} ${nextArgs[offset]}`, badgeLabel);
  } else {
    nextArgs.splice(offset, 0, badgeFormat, badgeLabel);
  }

  return bind.apply(consoleMethod, [console, ...nextArgs]) as () => unknown;
}

function createFabricContainerForHostConfig(options?: RasterRootOptions): FabricContainerNode {
  return createFabricContainer(options, getRasterNativeBinding());
}

function createFabricInstance(
  type: HostType,
  props: HostProps,
  rootContainer: FabricContainerNode,
  _hostContext: HostContext,
  internalHandle: { key?: string | null } | null
): FabricHostInstance {
  return createFabricHostNode(
    rootContainer.surfaceId,
    type,
    props,
    internalHandle?.key ?? null,
    getRasterNativeBinding()
  );
}

function createFabricTextInstance(
  text: string,
  rootContainer: FabricContainerNode
): FabricTextInstance {
  return createFabricTextNode(rootContainer.surfaceId, text, getRasterNativeBinding());
}

function createFabricContainerChildSet(container?: FabricContainerNode): FabricChildSet {
  const surfaceId = container?.surfaceId ?? currentFabricSurfaceId;
  if (surfaceId == null) {
    throw new Error("Raster Fabric renderer cannot create a child set without an active surface");
  }
  return createFabricChildSet({ $$typeof: "raster.native-container", surfaceId, parent: null }, getRasterNativeBinding());
}

function appendFabricChildToContainerChildSet(
  childSet: FabricChildSet,
  child: FabricHostInstance | FabricTextInstance
): void {
  void childSet;
  void child;
}

function appendInitialFabricHostChild(
  parent: FabricHostInstance,
  child: FabricHostInstance | FabricTextInstance
): void {
  appendInitialFabricChild(parent, child, getRasterNativeBinding());
}

function finalizeFabricContainerChildren(
  _container: FabricContainerNode,
  newChildren: FabricChildSet
): void {
  void newChildren;
}

function replaceFabricContainerChildren(
  container: FabricContainerNode,
  newChildren: FabricChildSet
): void {
  void container;
  void newChildren;
}

function detachDeletedFabricInstance(instance: FabricHostInstance | FabricTextInstance): void {
  deleteFabricNode(instance, getRasterNativeBinding());
}

export const __rasterFabricHostConfigInternals = {
  createContainer: createFabricContainerForHostConfig,
  createInstance: createFabricInstance,
  createTextInstance: createFabricTextInstance,
  appendInitialChild: appendInitialFabricHostChild,
  createContainerChildSet: createFabricContainerChildSet,
  appendChildToContainerChildSet: appendFabricChildToContainerChildSet,
  finalizeContainerChildren: finalizeFabricContainerChildren,
  replaceContainerChildren: replaceFabricContainerChildren,
  detachDeletedInstance: detachDeletedFabricInstance,
} satisfies {
  createContainer(options?: RasterRootOptions): FabricContainerNode;
  createInstance(
    type: HostType,
    props: HostProps,
    rootContainer: FabricContainerNode,
    hostContext: HostContext,
    internalHandle: { key?: string | null } | null
  ): FabricHostInstance;
  createTextInstance(text: string, rootContainer: FabricContainerNode): FabricTextInstance;
  appendInitialChild(parent: FabricHostInstance, child: FabricHostInstance | FabricTextInstance): void;
  createContainerChildSet(container?: FabricContainerNode): FabricChildSet;
  appendChildToContainerChildSet(childSet: FabricChildSet, child: FabricHostInstance | FabricTextInstance): void;
  finalizeContainerChildren(container: FabricContainerNode, newChildren: FabricChildSet): void;
  replaceContainerChildren(container: FabricContainerNode, newChildren: FabricChildSet): void;
  detachDeletedInstance(instance: FabricHostInstance | FabricTextInstance): void;
};

const commonHostConfig = {
  rendererPackageName: "react-raster",
  rendererVersion: rendererVersion(),
  extraDevToolsConfig: null,
  isPrimaryRenderer: true,
  warnsIfNotActing: false,

  getChildHostContext(parentHostContext: object) {
    return parentHostContext;
  },

  getCurrentEventPriority() {
    return DefaultEventPriority;
  },

  getCurrentUpdatePriority() {
    return currentUpdatePriority;
  },

  setCurrentUpdatePriority(priority: number) {
    currentUpdatePriority = priority;
  },

  resolveUpdatePriority() {
    return currentUpdatePriority === NoEventPriority
      ? DefaultEventPriority
      : currentUpdatePriority;
  },

  trackSchedulerEvent() {
    schedulerEvent = currentRasterEvent;
  },

  resolveEventType() {
    return currentRasterEvent && currentRasterEvent !== schedulerEvent
      ? currentRasterEvent.type
      : null;
  },

  resolveEventTimeStamp() {
    return currentRasterEvent && currentRasterEvent !== schedulerEvent
      ? currentRasterEvent.timeStamp
      : NO_EVENT_TIMESTAMP;
  },

  shouldAttemptEagerTransition() {
    return false;
  },
  detachDeletedInstance: detachDeletedFabricInstance,

  shouldSetTextContent() {
    return false;
  },

  prepareForCommit(containerInfo: FabricContainerNode) {
    prepareFabricCommit(containerInfo, getRasterNativeBinding());
    return null;
  },

  resetTextContent() {},
  beforeActiveInstanceBlur() {},
  afterActiveInstanceBlur() {},
  preparePortalMount() {},
  prepareScopeUpdate() {},
  getInstanceFromNode() {
    return null;
  },
  getInstanceFromScope() {
    return null;
  },
  scheduleTimeout: setTimeout,
  cancelTimeout: clearTimeout,
  noTimeout: -1,
  supportsMicrotasks: true,
  scheduleMicrotask(callback: () => void) {
    Promise.resolve().then(callback);
  },
  requestPostPaintCallback(callback: (time: number) => void) {
    callback(Date.now());
  },
  maySuspendCommit() {
    return false;
  },
  maySuspendCommitOnUpdate() {
    return false;
  },
  maySuspendCommitInSyncRender() {
    return false;
  },
  preloadInstance() {
    return true;
  },
  startSuspendingCommit() {},
  suspendInstance() {},
  suspendOnActiveViewTransition() {},
  waitForCommitToBeReady() {
    return null;
  },
  getSuspendedCommitReason() {
    return null;
  },
  resetFormInstance() {},
  bindToConsole(methodName: string, args: unknown[], environmentName?: string) {
    return bindConsoleMethod(methodName, args, environmentName);
  },
  NotPendingTransition,
  HostTransitionContext,
};

const fabricHostConfig = {
  ...commonHostConfig,
  supportsMutation: true,
  supportsHydration: false,
  supportsPersistence: false,

  getPublicInstance(instance: FabricPublicInstance) {
    return instance;
  },

  getRootHostContext(rootContainer: FabricContainerNode) {
    currentFabricSurfaceId = rootContainer.surfaceId;
    return rootHostContext;
  },

  createInstance: createFabricInstance,
  createTextInstance: createFabricTextInstance,
  appendInitialChild: appendInitialFabricHostChild,
  finalizeInitialChildren() {
    return false;
  },

  appendChild(parent: FabricHostInstance, child: FabricHostInstance | FabricTextInstance) {
    appendFabricChild(parent, child, getRasterNativeBinding());
  },
  appendChildToContainer(container: FabricContainerNode, child: FabricHostInstance | FabricTextInstance) {
    appendFabricChildToContainer(container, child, getRasterNativeBinding());
  },
  insertBefore(
    parent: FabricHostInstance,
    child: FabricHostInstance | FabricTextInstance,
    before: FabricHostInstance | FabricTextInstance
  ) {
    insertFabricChildBefore(parent, child, before, getRasterNativeBinding());
  },
  insertInContainerBefore(
    container: FabricContainerNode,
    child: FabricHostInstance | FabricTextInstance,
    before: FabricHostInstance | FabricTextInstance
  ) {
    insertFabricChildInContainerBefore(container, child, before, getRasterNativeBinding());
  },
  removeChild(parent: FabricHostInstance, child: FabricHostInstance | FabricTextInstance) {
    removeFabricChild(parent, child, getRasterNativeBinding());
  },
  removeChildFromContainer(container: FabricContainerNode, child: FabricHostInstance | FabricTextInstance) {
    removeFabricChildFromContainer(container, child, getRasterNativeBinding());
  },
  commitUpdate(
    instance: FabricHostInstance,
    type: HostType,
    _oldProps: HostProps,
    newProps: HostProps
  ) {
    updateFabricHostNode(instance, type, newProps, false, getRasterNativeBinding());
  },
  commitTextUpdate(instance: FabricTextInstance, _oldText: string, newText: string) {
    updateFabricTextNode(instance, newText, getRasterNativeBinding());
  },
  clearContainer() {},
  resetAfterCommit(containerInfo: FabricContainerNode) {
    resetFabricCommit(containerInfo, getRasterNativeBinding());
  },
} as unknown as FabricHostConfigBase & ReactCurrentHostConfigExtensions;

const fabricReconciler = Reconciler(fabricHostConfig) as unknown as FabricReconciler;
let currentEventReconciler: FabricReconciler = fabricReconciler;

function flushRasterWork(reconciler: FabricReconciler = currentEventReconciler): void {
  reconciler.flushSyncWork();

  const flushPassiveEffects = reconciler.flushPassiveEffects;
  if (flushPassiveEffects == null) {
    return;
  }

  let pass = 0;
  while (flushPassiveEffects.call(reconciler)) {
    reconciler.flushSyncWork();
    pass += 1;
    if (pass > 50) {
      throw new Error("Raster renderer exceeded passive effect flush limit");
    }
  }
}

function createFabricReconcilerContainer(containerInfo: FabricContainerNode): FabricContainer {
  const tag = ConcurrentRoot;
  const hydrationCallbacks = null;
  const isStrictMode = false;
  const concurrentUpdatesByDefaultOverride = null;
  const identifierPrefix = "";
  const onUncaughtError = fabricReconciler.defaultOnUncaughtError;
  const onCaughtError = fabricReconciler.defaultOnCaughtError;
  const onRecoverableError = fabricReconciler.defaultOnRecoverableError;
  const onDefaultTransitionIndicator = NOOP_DEFAULT_TRANSITION_INDICATOR;

  return fabricReconciler.createContainer(
    containerInfo,
    tag,
    hydrationCallbacks,
    isStrictMode,
    concurrentUpdatesByDefaultOverride,
    identifierPrefix,
    onUncaughtError,
    onCaughtError,
    onRecoverableError,
    onDefaultTransitionIndicator
  );
}

export function createFabricRoot(options?: RasterRootOptions): RasterDevRoot {
  const root = createFabricContainerForHostConfig(options);
  const container = createFabricReconcilerContainer(root);

  const runEvent = (
    handler: RasterEventHandler,
    payload: unknown,
    event: RasterEventMetadata | null
  ) => {
    const previousRasterEvent = currentRasterEvent;
    const previousGlobalEvent = rasterEventGlobal.event;
    const nextEvent = normalizeEventMetadata(event);
    currentRasterEvent = nextEvent;
    schedulerEvent = null;
    if (nextEvent == null) {
      delete rasterEventGlobal.event;
    } else {
      rasterEventGlobal.event = nextEvent;
    }

    try {
      const result = fabricReconciler.flushSyncFromReconciler(() =>
        fabricReconciler.discreteUpdates(handler, payload)
      );
      flushRasterWork(fabricReconciler);
      return result;
    } finally {
      currentRasterEvent = previousRasterEvent;
      if (previousGlobalEvent === undefined) {
        delete rasterEventGlobal.event;
      } else {
        rasterEventGlobal.event = previousGlobalEvent;
      }
    }
  };

  const rasterRoot: RasterDevRoot = {
    render(element: ReactElement | null) {
      const previousSurfaceId = currentFabricSurfaceId;
      currentFabricSurfaceId = root.surfaceId;
      currentEventReconciler = fabricReconciler;
      try {
        fabricReconciler.updateContainerSync(element, container, null, null);
        flushRasterWork(fabricReconciler);
      } finally {
        currentFabricSurfaceId = previousSurfaceId;
      }
    },
    clear() {
      const previousSurfaceId = currentFabricSurfaceId;
      currentFabricSurfaceId = root.surfaceId;
      currentEventReconciler = fabricReconciler;
      try {
        fabricReconciler.updateContainerSync(null, container, null, null);
        flushRasterWork(fabricReconciler);
        clearFabricSurface(root, getRasterNativeBinding());
      } finally {
        currentFabricSurfaceId = previousSurfaceId;
      }
    },
    __rasterRunEvent: runEvent,
    __rasterFlushSyncWork() {
      flushRasterWork(fabricReconciler);
    },
  };
  return rasterRoot;
}

export function createRoot(options?: RasterRootOptions): RasterRoot {
  if (rasterGlobal.__rasterDevReload === true) {
    if (rasterGlobal.__rasterDevRoot == null) {
      rasterGlobal.__rasterDevRoot = createFabricRoot(options);
    }
    rasterGlobal.__rasterRunEvent = rasterGlobal.__rasterDevRoot.__rasterRunEvent;
    rasterGlobal.__rasterFlushSyncWork = rasterGlobal.__rasterDevRoot.__rasterFlushSyncWork;
    return rasterGlobal.__rasterDevRoot;
  }
  const root = createFabricRoot(options);
  rasterGlobal.__rasterRunEvent = root.__rasterRunEvent;
  rasterGlobal.__rasterFlushSyncWork = root.__rasterFlushSyncWork;
  return root;
}

rasterGlobal.__rasterPrepareDevReload = () => {
  rasterGlobal.__rasterDevRoot?.clear();
};

rasterGlobal.__rasterFlushSyncWork ??= () => {
  flushRasterWork(currentEventReconciler);
};

rasterGlobal.__rasterRunEvent ??= (handler, payload, event) => {
  const previousRasterEvent = currentRasterEvent;
  const previousGlobalEvent = rasterEventGlobal.event;
  const nextEvent = normalizeEventMetadata(event);
  currentRasterEvent = nextEvent;
  schedulerEvent = null;
  if (nextEvent == null) {
    delete rasterEventGlobal.event;
  } else {
    rasterEventGlobal.event = nextEvent;
  }

  try {
    const result = currentEventReconciler.flushSyncFromReconciler(() =>
      currentEventReconciler.discreteUpdates(handler, payload)
    );
    flushRasterWork(currentEventReconciler);
    return result;
  } finally {
    currentRasterEvent = previousRasterEvent;
    if (previousGlobalEvent === undefined) {
      delete rasterEventGlobal.event;
    } else {
      rasterEventGlobal.event = previousGlobalEvent;
    }
  }
};
