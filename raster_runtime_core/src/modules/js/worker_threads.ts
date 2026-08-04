// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
//
// Minimal Node-compatible `worker_threads` surface for packages that probe
// `isMainThread` / MessageChannel. Full Worker spawning is fail-fast.

/** Always true: Raster runs as a single main thread. */
export const isMainThread = true;

/** No parent port in the main thread. */
export const parentPort = null;

/** No worker data in the main thread. */
export const workerData = undefined;

/** Thread id of the main thread. */
export const threadId = 0;

/**
 * Stub Worker — constructing throws (do not claim Worker compatibility).
 */
export class Worker {
  constructor(_filename?: string | URL, _options?: object) {
    throw new Error(
      "worker_threads.Worker is not implemented in raster_runtime yet"
    );
  }
}

type MessageListener = (value: unknown) => void;

/** Module-level environment data shared across the main thread (Node semantics). */
const environmentData = new Map<unknown, unknown>();

function toDataCloneError(err: unknown): DOMException {
  if (
    err &&
    typeof err === "object" &&
    "name" in err &&
    (err as { name?: string }).name === "DataCloneError"
  ) {
    return err as DOMException;
  }
  const message =
    err && typeof err === "object" && "message" in err
      ? String((err as { message?: unknown }).message ?? err)
      : String(err ?? "The object could not be cloned.");
  return new DOMException(message, "DataCloneError");
}

/**
 * Interconnected MessagePort pair (Node-compatible messaging).
 * `postMessage` always structured-clones; values are never passed by reference.
 */
export class MessagePort {
  onmessage: ((ev: { data: unknown }) => void) | null = null;
  onmessageerror: ((ev: unknown) => void) | null = null;
  #peer: MessagePort | null = null;
  #listeners = new Map<string, Set<MessageListener>>();
  #closed = false;

  /** Internal: wire peer after both ports are constructed. */
  _setPeer(peer: MessagePort) {
    this.#peer = peer;
  }

  start(): void {}

  close(): void {
    if (this.#closed) return;
    this.#closed = true;
    this.#dispatch("close", undefined);
  }

  /**
   * Structured-clone `message` and deliver asynchronously to the peer.
   * Uncloneable values throw `DataCloneError`. Closed ports drop delivery.
   */
  postMessage(message: unknown, transfer?: unknown[]): void {
    if (this.#closed) return;

    let transferOptions: { transfer: unknown[] } | undefined;
    if (transfer != null) {
      if (!Array.isArray(transfer)) {
        throw new TypeError(
          'The "transferList" argument must be an instance of Array.'
        );
      }
      if (transfer.length > 0) {
        transferOptions = { transfer };
      }
    }

    let cloned: unknown;
    try {
      cloned = transferOptions
        ? structuredClone(message, transferOptions as object)
        : structuredClone(message);
    } catch (err) {
      throw toDataCloneError(err);
    }

    const peer = this.#peer;
    if (!peer || peer.#closed) return;

    queueMicrotask(() => {
      // Closed port: stop delivery.
      if (peer.#closed) return;
      peer.dispatchMessage(cloned);
    });
  }

  /** Deliver a previously cloned message (listener exceptions are not swallowed). */
  dispatchMessage(data: unknown): void {
    this.#dispatch("message", data);
    if (typeof this.onmessage === "function") {
      this.onmessage({ data });
    }
  }

  /**
   * Node `port.ref()` — keep the event loop alive while the port is active.
   * Raster has no process-handle keepalive yet; retained for API compatibility
   * (returns `this`, does not pretend via unused internal flags).
   */
  ref(): this {
    return this;
  }

  /**
   * Node `port.unref()` — allow exit if this is the only remaining handle.
   * See `ref()`; no-op until a real keepalive exists.
   */
  unref(): this {
    return this;
  }

  addEventListener(type: string, listener: MessageListener): void {
    let set = this.#listeners.get(type);
    if (!set) {
      set = new Set();
      this.#listeners.set(type, set);
    }
    set.add(listener);
  }

  removeEventListener(type: string, listener: MessageListener): void {
    this.#listeners.get(type)?.delete(listener);
  }

  on(type: string, listener: MessageListener): this {
    this.addEventListener(type, listener);
    return this;
  }

  off(type: string, listener: MessageListener): this {
    this.removeEventListener(type, listener);
    return this;
  }

  #dispatch(type: string, data: unknown) {
    const set = this.#listeners.get(type);
    if (!set) return;
    // Do not swallow listener exceptions — they must surface to the host.
    for (const fn of [...set]) {
      fn(data);
    }
  }
}

/**
 * Pair of connected MessagePorts.
 */
export class MessageChannel {
  port1: MessagePort;
  port2: MessagePort;
  constructor() {
    this.port1 = new MessagePort();
    this.port2 = new MessagePort();
    this.port1._setPeer(this.port2);
    this.port2._setPeer(this.port1);
  }
}

/** Clone-on-get environment data (Node `worker_threads` API). */
export function getEnvironmentData(key: unknown): unknown {
  if (!environmentData.has(key)) {
    return undefined;
  }
  return structuredClone(environmentData.get(key));
}

export function setEnvironmentData(key: unknown, value: unknown): void {
  // Store a clone so later mutations of `value` do not affect the map entry.
  environmentData.set(key, structuredClone(value));
}

export function markAsUntransferable(_object: unknown): void {
  // no-op until transfer list integration is complete
}

export function isMarkedAsUntransferable(_object: unknown): boolean {
  return false;
}

export function moveMessagePortToContext(
  _port: unknown,
  _context: unknown
): never {
  throw new Error(
    "worker_threads.moveMessagePortToContext is not implemented"
  );
}

export function receiveMessageOnPort(_port: unknown): undefined {
  return undefined;
}

export default {
  isMainThread,
  parentPort,
  workerData,
  threadId,
  Worker,
  MessagePort,
  MessageChannel,
  getEnvironmentData,
  setEnvironmentData,
  markAsUntransferable,
  isMarkedAsUntransferable,
  moveMessagePortToContext,
  receiveMessageOnPort,
};
