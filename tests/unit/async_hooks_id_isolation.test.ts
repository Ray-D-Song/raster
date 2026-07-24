// Regression suite for async-resource ID isolation across providers.
//
// Background: TickObject / Microtask / Timeout previously each kept an
// independent counter starting at 1, so a microtask and a nextTick could
// both receive ID 1 and clobber each other's entry in the shared async-hooks
// id_map — producing `[["micro","tick"],["tick",null]]` instead of
// `[["micro","micro"],["tick","tick"]]`. These tests pin the fixed behavior
// where every provider draws from one process-wide allocator.
import {
  AsyncLocalStorage,
  AsyncResource,
  createHook,
  executionAsyncId,
} from "node:async_hooks";

// Drain all microtasks (queueMicrotask + process.nextTick) and one macrotask
// tick so every scheduled callback has run before assertions.
async function drain(tickMs = 20) {
  await new Promise((resolve) => setTimeout(resolve, tickMs));
}

describe("cross-provider ALS store isolation", () => {
  it("microtask and nextTick keep distinct stores", async () => {
    const als = new AsyncLocalStorage<string>();
    const seen: Record<string, string | undefined> = {};
    als.run("micro", () => {
      queueMicrotask(() => {
        seen.micro = als.getStore();
      });
    });
    als.run("tick", () => {
      process.nextTick(() => {
        seen.tick = als.getStore();
      });
    });
    await drain();
    expect(seen.micro).toBe("micro");
    expect(seen.tick).toBe("tick");
  });

  it("timer and microtask keep distinct stores", async () => {
    const als = new AsyncLocalStorage<string>();
    const seen: Record<string, string | undefined> = {};
    als.run("timer", () => {
      setTimeout(() => {
        seen.timer = als.getStore();
      }, 0);
    });
    als.run("micro", () => {
      queueMicrotask(() => {
        seen.micro = als.getStore();
      });
    });
    await drain();
    expect(seen.timer).toBe("timer");
    expect(seen.micro).toBe("micro");
  });

  it("timer and nextTick keep distinct stores", async () => {
    const als = new AsyncLocalStorage<string>();
    const seen: Record<string, string | undefined> = {};
    als.run("timer", () => {
      setTimeout(() => {
        seen.timer = als.getStore();
      }, 0);
    });
    als.run("tick", () => {
      process.nextTick(() => {
        seen.tick = als.getStore();
      });
    });
    await drain();
    expect(seen.timer).toBe("timer");
    expect(seen.tick).toBe("tick");
  });

  it("repeated scheduling of the same callback keeps distinct stores", async () => {
    const als = new AsyncLocalStorage<string>();
    const seen: (string | undefined)[] = [];
    function cb() {
      seen.push(als.getStore());
    }
    als.run("first", () => queueMicrotask(cb));
    als.run("second", () => queueMicrotask(cb));
    await drain();
    expect(seen).toEqual(["first", "second"]);
  });

  it("different ALS instances do not cross-contaminate", async () => {
    const a = new AsyncLocalStorage<string>();
    const b = new AsyncLocalStorage<number>();
    const seen: Record<string, string | number | undefined> = {};
    a.run("A", () => {
      queueMicrotask(() => {
        seen.a = a.getStore();
      });
    });
    b.run(42, () => {
      process.nextTick(() => {
        seen.b = b.getStore();
      });
    });
    await drain();
    expect(seen.a).toBe("A");
    expect(seen.b).toBe(42);
  });
});

describe("init/before/after/destroy id pairing across providers", () => {
  it("microtask and nextTick get distinct asyncIds with paired lifecycle", async () => {
    const typeById = new Map<number, string>();
    const events: Array<string> = [];
    const hook = createHook({
      init(asyncId, type) {
        if (type === "Microtask" || type === "TickObject") {
          typeById.set(asyncId, type);
          events.push(`${type}:${asyncId}:init`);
        }
      },
      before(asyncId) {
        const type = typeById.get(asyncId);
        if (type) events.push(`${type}:${asyncId}:before`);
      },
      after(asyncId) {
        const type = typeById.get(asyncId);
        if (type) events.push(`${type}:${asyncId}:after`);
      },
      destroy(asyncId) {
        const type = typeById.get(asyncId);
        if (type) events.push(`${type}:${asyncId}:destroy`);
      },
    });
    hook.enable();

    const als = new AsyncLocalStorage<string>();
    await als.run("ctx", async () => {
      queueMicrotask(() => {});
      process.nextTick(() => {});
      await drain();
    });

    hook.disable();

    const ids = Array.from(typeById.keys());
    expect(ids.length).toBe(2);
    // The core bug: two providers must not share the same id.
    expect(ids[0]).not.toBe(ids[1]);
    for (const id of ids) {
      const type = typeById.get(id)!;
      expect(events).toContain(`${type}:${id}:init`);
      expect(events).toContain(`${type}:${id}:before`);
      expect(events).toContain(`${type}:${id}:after`);
      expect(events).toContain(`${type}:${id}:destroy`);
    }
  });

  it("timer and microtask get distinct asyncIds with paired lifecycle", async () => {
    // Capture only the first Timeout and first Microtask so the timer that
    // `drain()` schedules internally is not counted as a third resource.
    let timerId = -1;
    let microId = -1;
    const events: Array<string> = [];
    const record = (label: string, id: number, phase: string) =>
      events.push(`${label}:${id}:${phase}`);
    const hook = createHook({
      init(asyncId, type) {
        if (type === "Timeout" && timerId === -1) {
          timerId = asyncId;
          record("Timeout", asyncId, "init");
        }
        if (type === "Microtask" && microId === -1) {
          microId = asyncId;
          record("Microtask", asyncId, "init");
        }
      },
      before(asyncId) {
        if (asyncId === timerId) record("Timeout", asyncId, "before");
        if (asyncId === microId) record("Microtask", asyncId, "before");
      },
      after(asyncId) {
        if (asyncId === timerId) record("Timeout", asyncId, "after");
        if (asyncId === microId) record("Microtask", asyncId, "after");
      },
      destroy(asyncId) {
        if (asyncId === timerId) record("Timeout", asyncId, "destroy");
        if (asyncId === microId) record("Microtask", asyncId, "destroy");
      },
    });
    hook.enable();

    const als = new AsyncLocalStorage<string>();
    await als.run("ctx", async () => {
      setTimeout(() => {}, 0);
      queueMicrotask(() => {});
      await drain();
    });

    hook.disable();

    expect(timerId).toBeGreaterThan(0);
    expect(microId).toBeGreaterThan(0);
    // The core bug: two providers must not share the same id.
    expect(timerId).not.toBe(microId);
    for (const [label, id] of [
      ["Timeout", timerId],
      ["Microtask", microId],
    ] as const) {
      expect(events).toContain(`${label}:${id}:init`);
      expect(events).toContain(`${label}:${id}:before`);
      expect(events).toContain(`${label}:${id}:after`);
      expect(events).toContain(`${label}:${id}:destroy`);
    }
  });
});

describe("interval self-clear lifecycle", () => {
  // A repeating interval whose callback calls clearInterval(id) must still
  // run After (and pop the async context) before destroy fires. The bug was
  // that clear removed the id_map entry immediately, so After could not find
  // it, pop_async_context() was skipped, and the execution stack leaked at
  // the destroyed interval's context. Node order: init -> before -> after ->
  // destroy.
  it("fires init/before/after/destroy in order when cleared inside its callback", async () => {
    const events: Array<string> = [];
    let intervalId = -1;
    const hook = createHook({
      init(asyncId, type) {
        if (type === "Interval" && intervalId === -1) {
          intervalId = asyncId;
          events.push("init");
        }
      },
      before(asyncId) {
        if (asyncId === intervalId) events.push("before");
      },
      after(asyncId) {
        if (asyncId === intervalId) events.push("after");
      },
      destroy(asyncId) {
        if (asyncId === intervalId) events.push("destroy");
      },
    });
    hook.enable();

    const als = new AsyncLocalStorage<string>();
    await als.run("ctx", async () => {
      const id = setInterval(() => {
        events.push("callback");
        clearInterval(id);
      }, 1);
      await drain(40);
    });

    hook.disable();
    expect(events).toEqual(["init", "before", "callback", "after", "destroy"]);
  });

  it("restores executionAsyncId off the interval context after a self-clear", async () => {
    let beforeId = -1;
    let intervalId = -1;
    const hook = createHook({
      init(asyncId, type) {
        if (type === "Interval" && intervalId === -1) intervalId = asyncId;
      },
      before(asyncId) {
        if (asyncId === intervalId) beforeId = executionAsyncId();
      },
    });
    hook.enable();

    const parent = new AsyncResource("PARENT");
    await parent.runInAsyncScope(async () => {
      const id = setInterval(() => {
        clearInterval(id);
      }, 1);
      await drain(40);
      // Without the fix the context stack leaked at the interval's id, so
      // executionAsyncId() stayed equal to intervalId. (It is not asserted
      // to equal the parent id here because `await` itself resumes in the
      // awaited promise's reaction context.)
      expect(executionAsyncId()).not.toBe(intervalId);
    });

    hook.disable();
    expect(intervalId).toBeGreaterThan(0);
    // During Before the execution context was the interval's, not the parent.
    expect(beforeId).toBe(intervalId);
  });

  it("ALS store does not leak to the outer scope after a self-clear", async () => {
    const als = new AsyncLocalStorage<string>();
    let storeDuringCallback: string | undefined;
    await als.run("inner", async () => {
      const id = setInterval(() => {
        storeDuringCallback = als.getStore();
        clearInterval(id);
      }, 1);
      await drain(40);
    });
    expect(storeDuringCallback).toBe("inner");
    // A leaked context stack would leave the store stuck at "inner".
    expect(als.getStore()).toBeUndefined();
  });
});
