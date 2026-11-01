import assert from "node:assert";
import nodeTest, {
  __resetTestPlanForTests,
  after,
  afterEach,
  before,
  beforeEach,
  describe,
  it,
  run,
  suite,
  test,
} from "node:test";

it("exports node:test surface", () => {
  expect(typeof test).toBe("function");
  expect(typeof describe).toBe("function");
  expect(typeof suite).toBe("function");
  expect(typeof run).toBe("function");
  expect(test).toBe(it);
  expect(describe).toBe(suite);
  expect(typeof before).toBe("function");
  expect(typeof after).toBe("function");
  expect(typeof beforeEach).toBe("function");
  expect(typeof afterEach).toBe("function");
  expect(nodeTest).toBe(test);
  expect(typeof test.skip).toBe("function");
  expect(typeof test.only).toBe("function");
  expect(typeof test.todo).toBe("function");
});

it("run() executes describe/tests with hooks", async () => {
  __resetTestPlanForTests();
  const order: string[] = [];

  before(() => {
    order.push("before");
  });
  beforeEach(() => {
    order.push("beforeEach");
  });
  afterEach(() => {
    order.push("afterEach");
  });
  after(() => {
    order.push("after");
  });

  describe("math", () => {
    test("adds", () => {
      order.push("test");
      assert.equal(1 + 1, 2);
    });
    test.skip("skipped", () => {
      throw new Error("should not run");
    });
  });

  const result = await run();
  expect(result.fail).toBe(0);
  expect(result.pass).toBe(1);
  expect(result.skip).toBe(1);
  expect(order).toEqual([
    "before",
    "beforeEach",
    "test",
    "afterEach",
    "after",
  ]);
});

it("top-level test() returns a promise that runs the test", async () => {
  __resetTestPlanForTests();
  let ran = false;
  await test("immediate", () => {
    ran = true;
  });
  expect(ran).toBe(true);
});

it("supports async tests and done callbacks", async () => {
  __resetTestPlanForTests();
  describe("async", () => {
    test("promise", async () => {
      await Promise.resolve();
      assert.ok(true);
    });
    test("done callback", (_t, done) => {
      queueMicrotask(() => done());
    });
  });
  const result = await run();
  expect(result.fail).toBe(0);
  expect(result.pass).toBe(2);
});
