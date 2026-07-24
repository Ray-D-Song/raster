import {
  AsyncLocalStorage,
  AsyncResource,
  createHook,
  executionAsyncId,
} from "node:async_hooks";
import { url as inspectorUrl } from "node:inspector";
import {
  setImmediate,
  clearImmediate,
  setTimeout,
  clearTimeout,
} from "node:timers";
import {
  setTimeout as setTimeoutP,
  setImmediate as setImmediateP,
} from "node:timers/promises";

declare function assert(condition: boolean): void;

assert(typeof AsyncLocalStorage === "function");
assert(typeof AsyncResource === "function");
assert(typeof createHook === "function");
assert(typeof executionAsyncId === "function");

const als = new AsyncLocalStorage<string>();
assert(als.getStore() === undefined);

assert(typeof inspectorUrl === "function");
assert(inspectorUrl() === undefined);

assert(typeof setImmediate === "function");
assert(typeof clearImmediate === "function");
assert(typeof setTimeout === "function");
assert(typeof clearTimeout === "function");

assert(typeof setTimeoutP === "function");
assert(typeof setImmediateP === "function");
