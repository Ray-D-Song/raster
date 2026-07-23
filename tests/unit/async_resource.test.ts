import defaultImport from "node:async_hooks";
import legacyImport from "async_hooks";
import dns from "node:dns";

const {
  AsyncResource,
  createHook,
  executionAsyncId,
  triggerAsyncId,
} = defaultImport;

let uniqueSeq = 0;
function uniqueType(prefix = "TEST_ASYNC_RESOURCE") {
  uniqueSeq += 1;
  return `${prefix}_${Date.now()}_${uniqueSeq}`;
}

async function waitForGcDestroy(
  predicate: () => boolean,
  label: string,
  maxAttempts = 40
) {
  for (let i = 0; i < maxAttempts; i++) {
    __gc();
    await new Promise((resolve) => setTimeout(resolve, 10));
    if (predicate()) {
      return;
    }
  }
  throw new Error(`${label}: condition not met after ${maxAttempts} GC attempts`);
}

it("exports AsyncResource on CJS, node: alias, and default", () => {
  expect(typeof AsyncResource).toBe("function");
  expect(defaultImport.AsyncResource).toBe(legacyImport.AsyncResource);
  expect(defaultImport.default?.AsyncResource ?? defaultImport.AsyncResource).toBe(
    AsyncResource
  );
  expect(legacyImport.AsyncResource).toBe(AsyncResource);
});

it("supports Undici-style AsyncResource inheritance", () => {
  class RequestHandler extends AsyncResource {
    constructor() {
      super("UNDICI_REQUEST");
    }
  }

  const handler = new RequestHandler();
  expect(handler instanceof AsyncResource).toBe(true);
  expect(handler instanceof RequestHandler).toBe(true);
  expect(typeof handler.asyncId()).toBe("number");
  expect(handler.asyncId()).toBeGreaterThan(1);
});

describe("constructor", () => {
  it("rejects missing or non-string type", () => {
    // @ts-expect-error intentional invalid call
    expect(() => new AsyncResource()).toThrow(TypeError);
    // @ts-expect-error intentional invalid call
    expect(() => new AsyncResource(1)).toThrow(TypeError);
    // @ts-expect-error intentional invalid call
    expect(() => new AsyncResource(null)).toThrow(TypeError);
  });

  it("creates resources with default options and unique ids", () => {
    const a = new AsyncResource("A");
    const b = new AsyncResource("B");
    expect(a.asyncId()).not.toBe(b.asyncId());
    expect(a.triggerAsyncId()).toBe(executionAsyncId());
  });

  it("accepts object options and numeric legacy options", () => {
    const withObject = new AsyncResource("OBJ", { triggerAsyncId: 42 });
    expect(withObject.triggerAsyncId()).toBe(42);

    // Runtime compatibility with Node's undocumented numeric options form.
    // @ts-expect-error numeric options are not in the public TS signature
    const withNumber = new AsyncResource("NUM", 7);
    expect(withNumber.triggerAsyncId()).toBe(7);

    const withMinusOne = new AsyncResource("MINUS", { triggerAsyncId: -1 });
    expect(withMinusOne.triggerAsyncId()).toBe(-1);
  });

  it("rejects invalid triggerAsyncId values", () => {
    expect(() => new AsyncResource("X", { triggerAsyncId: -2 })).toThrow(
      RangeError
    );
    expect(() => new AsyncResource("X", { triggerAsyncId: 1.5 })).toThrow(
      RangeError
    );
    expect(() => new AsyncResource("X", { triggerAsyncId: NaN })).toThrow(
      RangeError
    );
    expect(() => new AsyncResource("X", { triggerAsyncId: Infinity })).toThrow(
      RangeError
    );
    expect(() =>
      new AsyncResource("X", { triggerAsyncId: Number.MAX_SAFE_INTEGER + 1 })
    ).toThrow(RangeError);
  });
});

describe("runInAsyncScope", () => {
  it("sets execution context, receiver, args, and return value", () => {
    const resource = new AsyncResource("SCOPE", { triggerAsyncId: 99 });
    const outerId = executionAsyncId();
    const outerTrigger = triggerAsyncId();
    const receiver = { tag: "recv" };

    const result = resource.runInAsyncScope(
      function (this: typeof receiver, a: number, b: string) {
        expect(this).toBe(receiver);
        expect(a).toBe(1);
        expect(b).toBe("two");
        expect(executionAsyncId()).toBe(resource.asyncId());
        expect(triggerAsyncId()).toBe(resource.triggerAsyncId());
        return `${this.tag}:${a}:${b}`;
      },
      receiver,
      1,
      "two"
    );

    expect(result).toBe("recv:1:two");
    expect(executionAsyncId()).toBe(outerId);
    expect(triggerAsyncId()).toBe(outerTrigger);
  });

  it("restores context when the callback throws", () => {
    const resource = new AsyncResource("THROW");
    const outerId = executionAsyncId();
    const error = new Error("expected");

    expect(() =>
      resource.runInAsyncScope(() => {
        throw error;
      })
    ).toThrow(error);

    expect(executionAsyncId()).toBe(outerId);
  });

  it("supports nested scopes", () => {
    const outerId = executionAsyncId();
    const a = new AsyncResource("NEST_A");
    const b = new AsyncResource("NEST_B");
    const order: Array<string | number> = [];

    order.push(outerId);
    a.runInAsyncScope(() => {
      order.push(executionAsyncId());
      b.runInAsyncScope(() => {
        order.push(executionAsyncId());
      });
      order.push(executionAsyncId());
    });
    order.push(executionAsyncId());

    expect(order).toEqual([
      outerId,
      a.asyncId(),
      b.asyncId(),
      a.asyncId(),
      outerId,
    ]);
  });
});

describe("hook lifecycle", () => {
  it("emits init/before/after/destroy for a unique resource type", () => {
    const type = uniqueType();
    const events: Array<{ kind: string; asyncId?: number; type?: string }> = [];
    let resourceAsyncId = -1;
    let seenResource: object | null = null;
    const outerBeforeInit = executionAsyncId();

    const hook = createHook({
      init(asyncId, resourceType, trigger, resource) {
        if (resourceType !== type) return;
        events.push({ kind: "init", asyncId, type: resourceType });
        resourceAsyncId = asyncId;
        seenResource = resource;
        // init must not change the caller's execution ID
        expect(executionAsyncId()).toBe(outerBeforeInit);
        // Resource methods are fully available during init (Node-compatible).
        expect((resource as AsyncResource).asyncId()).toBe(asyncId);
        expect((resource as AsyncResource).triggerAsyncId()).toBe(trigger);
      },
      before(asyncId) {
        if (asyncId !== resourceAsyncId) return;
        events.push({ kind: "before", asyncId });
        expect(executionAsyncId()).toBe(resourceAsyncId);
      },
      after(asyncId) {
        if (asyncId !== resourceAsyncId) return;
        events.push({ kind: "after", asyncId });
        expect(executionAsyncId()).toBe(resourceAsyncId);
      },
      destroy(asyncId) {
        if (asyncId !== resourceAsyncId) return;
        events.push({ kind: "destroy", asyncId });
      },
    });
    hook.enable();

    const outerId = executionAsyncId();
    const resource = new AsyncResource(type, {
      requireManualDestroy: true,
      triggerAsyncId: 77,
    });
    expect(seenResource).toBe(resource);
    expect(resource.asyncId()).toBe(resourceAsyncId);
    expect(resource.triggerAsyncId()).toBe(77);
    expect(executionAsyncId()).toBe(outerId);

    resource.runInAsyncScope(() => {
      expect(executionAsyncId()).toBe(resource.asyncId());
    });
    expect(executionAsyncId()).toBe(outerId);

    const returned = resource.emitDestroy();
    expect(returned).toBe(resource);

    hook.disable();

    expect(events.map((e) => e.kind)).toEqual([
      "init",
      "before",
      "after",
      "destroy",
    ]);
  });
});

describe("trigger causality", () => {
  it("assigns timer triggerAsyncId from the enclosing AsyncResource", async () => {
    const type = uniqueType("TIMER_TRIGGER");
    let arAsyncId = -1;
    let timeoutTrigger = -1;

    const hook = createHook({
      init(asyncId, resourceType, trigger) {
        if (resourceType === type) {
          arAsyncId = asyncId;
        } else if (
          resourceType === "Timeout" &&
          arAsyncId !== -1 &&
          trigger === arAsyncId &&
          timeoutTrigger === -1
        ) {
          timeoutTrigger = trigger;
        }
      },
    });
    hook.enable();

    const resource = new AsyncResource(type, { triggerAsyncId: 77 });
    expect(resource.asyncId()).toBe(arAsyncId);
    expect(resource.triggerAsyncId()).toBe(77);

    await new Promise<void>((resolve) => {
      resource.runInAsyncScope(() => {
        setTimeout(resolve, 0);
      });
    });

    hook.disable();
    expect(timeoutTrigger).toBe(arAsyncId);
  });

  it("assigns nested timer triggerAsyncId from the innermost AsyncResource", async () => {
    const typeA = uniqueType("NEST_TIMER_A");
    const typeB = uniqueType("NEST_TIMER_B");
    let idA = -1;
    let idB = -1;
    let timeoutTrigger = -1;

    const hook = createHook({
      init(asyncId, resourceType, trigger) {
        if (resourceType === typeA) {
          idA = asyncId;
        } else if (resourceType === typeB) {
          idB = asyncId;
        } else if (
          resourceType === "Timeout" &&
          idB !== -1 &&
          trigger === idB &&
          timeoutTrigger === -1
        ) {
          timeoutTrigger = trigger;
        }
      },
    });
    hook.enable();

    const a = new AsyncResource(typeA);
    const b = new AsyncResource(typeB);

    await new Promise<void>((resolve) => {
      a.runInAsyncScope(() => {
        b.runInAsyncScope(() => {
          setTimeout(resolve, 0);
        });
      });
    });

    hook.disable();
    expect(timeoutTrigger).toBe(idB);
    expect(timeoutTrigger).not.toBe(idA);
  });

  it("assigns DNS lookup triggerAsyncId from the enclosing AsyncResource", async () => {
    const type = uniqueType("DNS_TRIGGER");
    let arAsyncId = -1;
    let dnsTrigger = -1;

    const hook = createHook({
      init(asyncId, resourceType, trigger) {
        if (resourceType === type) {
          arAsyncId = asyncId;
        } else if (
          resourceType === "GETADDRINFOREQWRAP" &&
          arAsyncId !== -1 &&
          trigger === arAsyncId &&
          dnsTrigger === -1
        ) {
          dnsTrigger = trigger;
        }
      },
    });
    hook.enable();

    const resource = new AsyncResource(type, { triggerAsyncId: 88 });

    await new Promise<void>((resolve, reject) => {
      resource.runInAsyncScope(() => {
        dns.lookup("localhost", (err) => {
          if (err) reject(err);
          else resolve();
        });
      });
    });

    hook.disable();
    expect(dnsTrigger).toBe(arAsyncId);
  });

  it("assigns Promise triggerAsyncId from the enclosing AsyncResource when no parent", async () => {
    const type = uniqueType("PROMISE_TRIGGER");
    let arAsyncId = -1;
    let promiseTrigger = -1;

    const hook = createHook({
      init(asyncId, resourceType, trigger) {
        if (resourceType === type) {
          arAsyncId = asyncId;
        } else if (
          resourceType === "PROMISE" &&
          arAsyncId !== -1 &&
          trigger === arAsyncId &&
          promiseTrigger === -1
        ) {
          promiseTrigger = trigger;
        }
      },
    });
    hook.enable();

    const resource = new AsyncResource(type);

    await resource.runInAsyncScope(async () => {
      await Promise.resolve(1);
    });

    hook.disable();
    expect(promiseTrigger).toBe(arAsyncId);
  });
});

describe("bind", () => {
  it("binds instance methods with fixed or dynamic this", () => {
    const resource = new AsyncResource("BIND");
    const fixed = { id: 1 };
    const dynamic = { id: 2 };

    function fn(this: { id: number }, x: number) {
      return this.id + x;
    }

    const boundFixed = resource.bind(fn, fixed);
    expect(boundFixed(10)).toBe(11);
    expect(boundFixed.length).toBe(fn.length);
    expect((boundFixed as any).asyncResource).toBe(resource);

    const boundDynamic = resource.bind(fn);
    expect(boundDynamic.call(dynamic, 3)).toBe(5);

    const error = new Error("bind-error");
    const boundThrow = resource.bind(() => {
      throw error;
    });
    expect(() => boundThrow()).toThrow(error);

    // asyncResource setter only changes the visible value, not the closed-over resource.
    const other = new AsyncResource("OTHER");
    (boundFixed as any).asyncResource = other;
    expect((boundFixed as any).asyncResource).toBe(other);

    const outerId = executionAsyncId();
    const boundScope = resource.bind(() => executionAsyncId());
    expect(boundScope()).toBe(resource.asyncId());
    expect(executionAsyncId()).toBe(outerId);
  });

  it("static bind creates a fresh AsyncResource", () => {
    const bound = AsyncResource.bind(function named(a: number) {
      return a * 2;
    }, "STATIC_BIND");

    expect(bound(4)).toBe(8);
    expect(bound.length).toBe(1);
    expect(typeof (bound as any).asyncResource).toBe("object");
    expect((bound as any).asyncResource instanceof AsyncResource).toBe(true);
    expect((bound as any).asyncResource.asyncId()).toBeGreaterThan(1);

    const outerId = executionAsyncId();
    const boundId = AsyncResource.bind(() => executionAsyncId());
    const id = boundId();
    expect(id).not.toBe(outerId);
    expect(executionAsyncId()).toBe(outerId);
  });
});

describe("automatic destroy", () => {
  it("fires destroy via FinalizationRegistry when requireManualDestroy is false", async () => {
    const type = uniqueType("AUTO_DESTROY");
    const destroyed: number[] = [];
    let asyncId = -1;

    const hook = createHook({
      init(id, resourceType) {
        if (resourceType === type) {
          asyncId = id;
        }
      },
      destroy(id) {
        if (id === asyncId) {
          destroyed.push(id);
        }
      },
    });
    hook.enable();

    (() => {
      new AsyncResource(type, { requireManualDestroy: false });
    })();

    await waitForGcDestroy(
      () => destroyed.length >= 1,
      `auto-destroy for ${type} (asyncId=${asyncId}, events=${JSON.stringify(destroyed)})`
    );

    hook.disable();
    expect(destroyed).toEqual([asyncId]);
  });

  it("does not auto-destroy after emitDestroy", async () => {
    const type = uniqueType("MANUAL_THEN_GC");
    const destroyed: number[] = [];
    let asyncId = -1;

    const hook = createHook({
      init(id, resourceType) {
        if (resourceType === type) {
          asyncId = id;
        }
      },
      destroy(id) {
        if (id === asyncId) {
          destroyed.push(id);
        }
      },
    });
    hook.enable();

    (() => {
      const resource = new AsyncResource(type, { requireManualDestroy: false });
      resource.emitDestroy();
    })();

    expect(destroyed).toEqual([asyncId]);

    // Give GC a chance to fire a second destroy if the flag failed.
    for (let i = 0; i < 10; i++) {
      __gc();
      await new Promise((resolve) => setTimeout(resolve, 10));
    }

    hook.disable();
    expect(destroyed).toEqual([asyncId]);
  });

  it("skips auto-destroy when requireManualDestroy is true", async () => {
    const type = uniqueType("REQUIRE_MANUAL");
    const destroyed: number[] = [];
    let asyncId = -1;
    let resource: InstanceType<typeof AsyncResource> | null = null;

    const hook = createHook({
      init(id, resourceType) {
        if (resourceType === type) {
          asyncId = id;
        }
      },
      destroy(id) {
        if (id === asyncId) {
          destroyed.push(id);
        }
      },
    });
    hook.enable();

    resource = new AsyncResource(type, { requireManualDestroy: true });
    const id = resource.asyncId();
    resource = null;

    for (let i = 0; i < 15; i++) {
      __gc();
      await new Promise((resolve) => setTimeout(resolve, 10));
    }

    expect(destroyed).toEqual([]);

    // Explicit destroy still works for a requireManualDestroy resource.
    const manual = new AsyncResource(type + "_EXPLICIT", {
      requireManualDestroy: true,
      triggerAsyncId: -1,
    });
    const explicitId = manual.asyncId();
    const explicitDestroyed: number[] = [];
    const hook2 = createHook({
      destroy(dId) {
        if (dId === explicitId) {
          explicitDestroyed.push(dId);
        }
      },
    });
    hook2.enable();
    manual.emitDestroy();
    hook2.disable();
    hook.disable();

    expect(explicitDestroyed).toEqual([explicitId]);
    expect(id).toBe(asyncId);
  });

  it("registers auto-destroy when destroy hook is enabled inside init", async () => {
    const type = uniqueType("INIT_ENABLE_DESTROY");
    const destroyed: number[] = [];
    let asyncId = -1;

    const destroyHook = createHook({
      destroy(id) {
        if (id === asyncId) {
          destroyed.push(id);
        }
      },
    });

    const initHook = createHook({
      init(id, resourceType) {
        if (resourceType === type) {
          asyncId = id;
          destroyHook.enable();
        }
      },
    });
    initHook.enable();

    (() => {
      new AsyncResource(type, { requireManualDestroy: false });
    })();

    await waitForGcDestroy(
      () => destroyed.length >= 1,
      `init-enable-destroy for ${type} (asyncId=${asyncId}, events=${JSON.stringify(destroyed)})`
    );

    initHook.disable();
    destroyHook.disable();
    expect(destroyed).toEqual([asyncId]);
  });

  it("does not auto-destroy when destroy hook is disabled inside init", async () => {
    const type = uniqueType("INIT_DISABLE_DESTROY");
    const destroyed: number[] = [];
    let asyncId = -1;

    const destroyHook = createHook({
      destroy(id) {
        if (id === asyncId) {
          destroyed.push(id);
        }
      },
    });
    destroyHook.enable();

    const initHook = createHook({
      init(id, resourceType) {
        if (resourceType === type) {
          asyncId = id;
          destroyHook.disable();
        }
      },
    });
    initHook.enable();

    (() => {
      new AsyncResource(type, { requireManualDestroy: false });
    })();

    // Re-enable after construction — must not retroactively register this resource.
    destroyHook.enable();

    for (let i = 0; i < 15; i++) {
      __gc();
      await new Promise((resolve) => setTimeout(resolve, 10));
    }

    initHook.disable();
    destroyHook.disable();
    expect(destroyed).toEqual([]);
    expect(asyncId).toBeGreaterThan(1);
  });
});
