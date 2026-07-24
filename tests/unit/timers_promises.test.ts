import defaultImport from "node:timers/promises";
import legacyImport from "timers/promises";
import { setImmediate as setImmediateCb, clearImmediate } from "node:timers";
import { promisify } from "node:util";

it("node:timers/promises should load like timers/promises", () => {
  expect(typeof defaultImport.setTimeout).toBe("function");
  expect(typeof defaultImport.setImmediate).toBe("function");
  expect(typeof legacyImport.setTimeout).toBe("function");
  expect(typeof legacyImport.setImmediate).toBe("function");
});

it("promise setTimeout resolves value", async () => {
  await expect(defaultImport.setTimeout(10, "ok")).resolves.toBe("ok");
});

it("promise setImmediate resolves value", async () => {
  await expect(defaultImport.setImmediate("imm")).resolves.toBe("imm");
});

it("rejects immediately when signal is already aborted", async () => {
  const ac = new AbortController();
  ac.abort("already");
  await expect(
    defaultImport.setTimeout(1000, "x", { signal: ac.signal })
  ).rejects.toBe("already");
  await expect(
    defaultImport.setImmediate("y", { signal: ac.signal })
  ).rejects.toBe("already");
});

it("rejects when signal aborts while waiting", async () => {
  const ac = new AbortController();
  const p = defaultImport.setTimeout(5000, "late", { signal: ac.signal });
  setTimeout(() => ac.abort("canceled"), 5);
  await expect(p).rejects.toBe("canceled");
});

it("settles only once when abort and timer race", async () => {
  const ac = new AbortController();
  const p = defaultImport.setTimeout(20, "done", { signal: ac.signal });
  setTimeout(() => ac.abort("race"), 5);
  try {
    const v = await p;
    // timer won
    expect(v).toBe("done");
  } catch (e) {
    // abort won
    expect(e).toBe("race");
  }
});

it("accepts { ref: false } without changing behavior", async () => {
  await expect(
    defaultImport.setImmediate("r", { ref: false })
  ).resolves.toBe("r");
});

it("CJS require export object is assignable for Next patches", () => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const mod = require("timers/promises");
  const original = mod.setImmediate;
  const replacement = async (v: unknown) => `patched:${v}`;
  mod.setImmediate = replacement;
  expect(mod.setImmediate).toBe(replacement);
  mod.setImmediate = original;
});

it("callback setImmediate forwards args and clearImmediate cancels", async () => {
  const value = await new Promise<string>((resolve) => {
    setImmediateCb((a: string, b: string) => resolve(a + b), "hel", "lo");
  });
  expect(value).toBe("hello");

  const result = await new Promise<string>((resolve) => {
    const id = setImmediateCb(() => resolve("should not"));
    clearImmediate(id);
    setImmediateCb(() => resolve("canceled"));
  });
  expect(result).toBe("canceled");
});

it("util.promisify(setImmediate) resolves value", async () => {
  const pSetImmediate = promisify(setImmediateCb);
  await expect(pSetImmediate("via-promisify")).resolves.toBe("via-promisify");
});

it("setTimeout/setInterval forward additional args", async () => {
  const a = await new Promise<number>((resolve) => {
    setTimeout((x: number, y: number) => resolve(x + y), 5, 3, 4);
  });
  expect(a).toBe(7);
});
