import defaultImport from "node:util";
import legacyImport from "util";

import { EventEmitter } from "node:events";

it("node:util should be the same as util", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const { inherits, promisify } = defaultImport;

describe("inherits", () => {
  it("should be inheritable parent classes", () => {
    function MyStream() {
      EventEmitter.call(this);
    }

    inherits(MyStream, EventEmitter);

    const stream = new MyStream();

    expect(stream instanceof EventEmitter).toBeTruthy();
    expect(MyStream.super_).toEqual(EventEmitter);
  });
});

describe("promisify", () => {
  it("should exist on named and default exports", async () => {
    const named = await import("util");
    expect(typeof promisify).toBe("function");
    expect(named.promisify).toBe(promisify);
    expect(named.default.promisify).toBe(promisify);
  });

  it("should resolve on callback success", async () => {
    const fn = (x: number, cb: (err: Error | null, value?: number) => void) => {
      cb(null, x + 1);
    };
    await expect(promisify(fn)(41)).resolves.toBe(42);
  });

  it("should reject on callback error", async () => {
    const err = new Error("boom");
    const fn = (_cb: (err: Error | null) => void) => {
      _cb(err);
    };
    await expect(promisify(fn)()).rejects.toBe(err);
  });

  it("should reject when the original throws synchronously", async () => {
    const fn = () => {
      throw new TypeError("sync fail");
    };
    await expect(promisify(fn)()).rejects.toThrow(/sync fail/);
  });

  it("should preserve this for method calls", async () => {
    const obj = {
      value: 7,
      method(cb: (err: Error | null, value?: number) => void) {
        cb(null, this.value);
      },
    };
    const bound = promisify(obj.method);
    await expect(bound.call(obj)).resolves.toBe(7);
  });

  it("should throw TypeError for non-function arguments", () => {
    expect(() => promisify(null as any)).toThrow(TypeError);
    expect(() => promisify(1 as any)).toThrow(
      /The "original" argument must be of type function/
    );
  });
});
