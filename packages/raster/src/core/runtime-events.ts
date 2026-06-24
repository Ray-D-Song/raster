type RasterRuntimeEventListener = (payload: unknown) => void;

type RasterRuntimeEventGlobal = typeof globalThis & {
  __rasterDispatchRuntimeEventJson?: (name: string, payloadJson: string) => void;
  __rasterRuntimeEventListeners?: Map<string, Set<RasterRuntimeEventListener>>;
};

const runtimeEventGlobal = globalThis as RasterRuntimeEventGlobal;
const listeners = (runtimeEventGlobal.__rasterRuntimeEventListeners ??= new Map());

export function addRasterRuntimeEventListener(name: string, listener: RasterRuntimeEventListener): () => void {
  const set = listeners.get(name) ?? new Set<RasterRuntimeEventListener>();
  set.add(listener);
  listeners.set(name, set);
  return () => {
    set.delete(listener);
    if (set.size === 0) listeners.delete(name);
  };
}

export function dispatchRasterRuntimeEvent(name: string, payload: unknown): void {
  const set = listeners.get(name);
  if (set == null) return;
  for (const listener of [...set]) {
    listener(payload);
  }
}

export function resetRasterRuntimeEventListeners(): void {
  listeners.clear();
}

runtimeEventGlobal.__rasterDispatchRuntimeEventJson ??= (name, payloadJson) => {
  dispatchRasterRuntimeEvent(name, JSON.parse(payloadJson));
};
