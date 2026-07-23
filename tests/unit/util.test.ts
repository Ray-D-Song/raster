import defaultImport from "node:util";
import legacyImport from "util";

import { EventEmitter } from "node:events";

it("node:util should be the same as util", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const { inherits, promisify, inspect, formatWithOptions, debuglog, toUSVString, types } =
  defaultImport;

it("format does not append trailing newlines", () => {
  expect(defaultImport.format("a", "b")).toBe("a b");
  expect(defaultImport.format("a", "b")).not.toContain("\n");
});

it("exposes inspect.custom and toUSVString", () => {
  expect(inspect.custom).toBe(Symbol.for("nodejs.util.inspect.custom"));
  expect(toUSVString("a\uD800b")).toBe("a\uFFFDb");
  expect(toUSVString("a\uD800\uDC00b")).toBe("a\uD800\uDC00b");
});

it("supports debuglog and util.types predicates", () => {
  const logger = debuglog("TEST");
  expect(typeof logger).toBe("function");
  expect(typeof logger.enabled).toBe("boolean");
  expect(typeof defaultImport.debug).toBe("function");

  expect(types.isSharedArrayBuffer()).toBe(false);
  expect(types.isUint8Array(new Uint8Array())).toBe(true);
  expect(types.isDataView(new DataView(new ArrayBuffer(1)))).toBe(true);
});

it("formatWithOptions formats multiple values", () => {
  expect(formatWithOptions({}, "a", "b")).toBe("a b");
  expect(formatWithOptions({ colors: true }, "%s:%s", "foo")).toBe("foo:%s");
});

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
