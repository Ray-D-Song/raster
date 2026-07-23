import {
  AsyncResource,
  createHook,
  executionAsyncId,
  triggerAsyncId,
  type AsyncResourceOptions,
  type HookCallbacks,
} from "async_hooks";
import {
  AsyncResource as NodeAsyncResource,
  executionAsyncId as nodeExecutionAsyncId,
} from "node:async_hooks";

declare function assertResource(r: AsyncResource): void;
declare function assertNumber(n: number): void;
declare function assertThis(r: AsyncResource): void;

const opts: AsyncResourceOptions = {
  triggerAsyncId: 1,
  requireManualDestroy: true,
};

const resource = new AsyncResource("SMOKE", opts);
assertResource(resource);
assertNumber(resource.asyncId());
assertNumber(resource.triggerAsyncId());
assertThis(resource.emitDestroy());

const result: string = resource.runInAsyncScope(
  function (this: { tag: string }, a: number, b: string) {
    return this.tag + a + b;
  },
  { tag: "x" },
  1,
  "y"
);
void result;

// thisArg is optional (Node-compatible).
const noThisArg: number = resource.runInAsyncScope(() => 42);
void noThisArg;

function handler(this: { id: number }, value: number): number {
  return this.id + value;
}

const bound = resource.bind(handler, { id: 10 });
assertNumber(bound(5));

const staticBound = AsyncResource.bind(
  function (this: { n: number }) {
    return this.n;
  },
  "bound-smoke",
  { n: 1 }
);
assertNumber(staticBound());

const callbacks: HookCallbacks = {
  init(asyncId, type, trigger, res) {
    assertNumber(asyncId);
    void type;
    assertNumber(trigger);
    void res;
  },
  before(asyncId) {
    assertNumber(asyncId);
  },
  after(asyncId) {
    assertNumber(asyncId);
  },
  destroy(asyncId) {
    assertNumber(asyncId);
  },
  promiseResolve(asyncId) {
    assertNumber(asyncId);
  },
};

const hook = createHook(callbacks);
hook.enable().disable();

assertNumber(executionAsyncId());
assertNumber(triggerAsyncId());
assertNumber(nodeExecutionAsyncId());

class RequestHandler extends NodeAsyncResource {
  constructor() {
    super("UNDICI_REQUEST");
  }
}

assertResource(new RequestHandler());
