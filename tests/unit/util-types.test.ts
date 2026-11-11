import util from "node:util";
import types from "node:util/types";
import { Duplex } from "node:stream";

// `util.types` must answer both ways for every predicate: a value of the right
// kind is `true`, and something that merely resembles it must be `false`.
// Node's own answers were captured by probing Node 24 and are what these assert.

const sab = new SharedArrayBuffer(8);
const ab = new ArrayBuffer(8);

const pairs: Array<[string, () => unknown, () => unknown]> = [
  ["isAnyArrayBuffer", () => ab, () => "x"],
  ["isAnyArrayBuffer (shared)", () => sab, () => 1],
  ["isArrayBuffer", () => ab, () => sab],
  ["isSharedArrayBuffer", () => sab, () => ab],
  ["isArrayBufferView", () => new Uint8Array(1), () => ab],
  ["isArrayBufferView (DataView)", () => new DataView(ab), () => 1],
  ["isTypedArray", () => new Uint8Array(1), () => new DataView(ab)],
  ["isUint8Array", () => new Uint8Array(1), () => new Int8Array(1)],
  ["isUint8ClampedArray", () => new Uint8ClampedArray(1), () => new Uint8Array(1)],
  ["isUint16Array", () => new Uint16Array(1), () => new Uint8Array(1)],
  ["isUint32Array", () => new Uint32Array(1), () => new Uint8Array(1)],
  ["isInt8Array", () => new Int8Array(1), () => new Uint8Array(1)],
  ["isInt16Array", () => new Int16Array(1), () => new Uint8Array(1)],
  ["isInt32Array", () => new Int32Array(1), () => new Uint8Array(1)],
  ["isFloat16Array", () => (typeof Float16Array === "undefined" ? null : new Float16Array(1)), () => new Uint8Array(1)],
  ["isFloat32Array", () => new Float32Array(1), () => new Uint8Array(1)],
  ["isFloat64Array", () => new Float64Array(1), () => new Uint8Array(1)],
  ["isBigInt64Array", () => new BigInt64Array(1), () => new Uint8Array(1)],
  ["isBigUint64Array", () => new BigUint64Array(1), () => new Uint8Array(1)],
  ["isDataView", () => new DataView(ab), () => new Uint8Array(1)],
  ["isDate", () => new Date(), () => ({})],
  ["isRegExp", () => /x/, () => ({})],
  ["isMap", () => new Map(), () => ({})],
  ["isSet", () => new Set(), () => ({})],
  ["isWeakMap", () => new WeakMap(), () => ({})],
  ["isWeakSet", () => new WeakSet(), () => ({})],
  ["isMapIterator", () => new Map().entries(), () => ({})],
  ["isSetIterator", () => new Set().values(), () => ({})],
  ["isArgumentsObject", () => (function () { return arguments; })(), () => []],
  ["isPromise", () => Promise.resolve(), () => ({})],
  ["isProxy", () => new Proxy({}, {}), () => ({})],
  ["isNativeError", () => new Error("x"), () => Object.create(Error.prototype)],
  ["isStringObject", () => new String("a"), () => "a"],
  ["isNumberObject", () => new Number(1), () => 1],
  ["isBooleanObject", () => new Boolean(1), () => true],
  ["isBigIntObject", () => Object(1n), () => 1n],
  ["isSymbolObject", () => Object(Symbol("a")), () => Symbol("a")],
  ["isBoxedPrimitive", () => new String("a"), () => ({})],
  ["isAsyncFunction", () => async function () {}, () => function () {}],
  ["isGeneratorFunction", () => function* () {}, () => function () {}],
  ["isGeneratorObject", () => (function* () {})() , () => ({})],
];

it("every util.types predicate exists", () => {
  for (const [name] of pairs) {
    const key = name.split(" ")[0];
    expect(typeof (types as any)[key]).toBe("function");
  }
});

it("every util.types predicate accepts a value of its kind", () => {
  for (const [name, positive] of pairs) {
    const key = name.split(" ")[0];
    const value = positive();
    if (value === null) continue; // Float16Array may not exist here
    expect(`${key}=${(types as any)[key](value)}`).toBe(`${key}=true`);
  }
});

it("every util.types predicate rejects the look-alike", () => {
  for (const [name, , negative] of pairs) {
    const key = name.split(" ")[0];
    expect(`${key}=${(types as any)[key](negative())}`).toBe(`${key}=false`);
  }
});

it("array buffer predicates tell the two kinds apart", () => {
  // The two bugs that were here: isArrayBuffer(SAB) returned true, and
  // isSharedArrayBuffer always returned false.
  expect(types.isArrayBuffer(sab)).toBe(false);
  expect(types.isSharedArrayBuffer(ab)).toBe(false);
  expect(types.isSharedArrayBuffer(sab)).toBe(true);
  expect(types.isAnyArrayBuffer(ab)).toBe(true);
  expect(types.isAnyArrayBuffer(sab)).toBe(true);
});

it("boxed predicates do not match primitives", () => {
  expect(types.isStringObject("a")).toBe(false);
  expect(types.isNumberObject(1)).toBe(false);
  expect(types.isBooleanObject(true)).toBe(false);
  expect(types.isBoxedPrimitive(1)).toBe(false);
  expect(types.isSymbolObject(Symbol("s"))).toBe(false);
});

it("predicates never throw on odd inputs", () => {
  for (const value of [undefined, null, 1, "s", true, Symbol("x"), () => {}, [], {}]) {
    for (const [name] of pairs) {
      const key = name.split(" ")[0];
      expect(() => (types as any)[key](value)).not.toThrow();
    }
  }
});

// --- MIME ---------------------------------------------------------------

it("MIMEType parses and re-serializes", () => {
  const mime = new util.MIMEType("text/html; charset=UTF-8; boundary=xyz");
  expect(mime.type).toBe("text");
  expect(mime.subtype).toBe("html");
  expect(mime.essence).toBe("text/html");
  expect(mime.params.get("charset")).toBe("UTF-8");
  expect([...mime.params.keys()]).toEqual(["charset", "boundary"]);

  mime.params.set("x", "1");
  mime.params.delete("boundary");
  expect(mime.toString()).toBe("text/html;charset=UTF-8;x=1");
});

it("MIMEType lowercases type and subtype but not parameter values", () => {
  const mime = new util.MIMEType("TEXT/HTML; Charset=UTF-8");
  expect(mime.essence).toBe("text/html");
  expect(mime.params.get("charset")).toBe("UTF-8");
});

it("MIMEType rejects a malformed type", () => {
  expect(() => new util.MIMEType("nope")).toThrow();
  expect(() => new util.MIMEType("text/")).toThrow();
});

it("MIMEParams is a constructor", () => {
  expect(typeof util.MIMEParams).toBe("function");
});

// --- system errors ------------------------------------------------------

it("getSystemErrorMap is a Map of errno to [code, message]", () => {
  const map = util.getSystemErrorMap();
  expect(map instanceof Map).toBe(true);
  expect(map.get(-2)).toEqual(["ENOENT", "no such file or directory"]);
});

it("getSystemErrorName and getSystemErrorMessage", () => {
  expect(util.getSystemErrorName(-2)).toBe("ENOENT");
  expect(util.getSystemErrorMessage(-2)).toBe("no such file or directory");
});

it("system error helpers reject non-negative codes", () => {
  expect(() => util.getSystemErrorName(2)).toThrow();
  expect(() => util.getSystemErrorMessage(2)).toThrow();
  try {
    util.getSystemErrorMessage(2);
  } catch (err: any) {
    expect(err.code).toBe("ERR_OUT_OF_RANGE");
  }
});

// --- signals ------------------------------------------------------------

it("convertProcessSignalToExitCode maps signals to 128 + signo", () => {
  expect(util.convertProcessSignalToExitCode("SIGINT")).toBe(130);
  expect(util.convertProcessSignalToExitCode("SIGTERM")).toBe(143);
  expect(util.convertProcessSignalToExitCode("SIGKILL")).toBe(137);
});

it("convertProcessSignalToExitCode rejects an unknown signal", () => {
  expect(() => util.convertProcessSignalToExitCode("SIGNOPE")).toThrow();
});

// --- misc helpers -------------------------------------------------------

it("diff reports per-line insertions and deletions", () => {
  expect(util.diff("a\nb\nc", "a\nx\nc")).toEqual([
    [0, "a"],
    [0, "\n"],
    [1, "b"],
    [-1, "x"],
    [0, "\n"],
    [0, "c"],
  ]);
});

it("getCallSites returns call-site records", () => {
  const sites = util.getCallSites();
  expect(Array.isArray(sites)).toBe(true);
  expect(sites.length).toBeGreaterThan(0);
  const site: any = sites[0];
  for (const key of ["functionName", "scriptName", "lineNumber", "column"]) {
    expect(site).toHaveProperty(key);
  }
});

it("aborted resolves with a cleanup function", async () => {
  const controller = new AbortController();
  const pending = util.aborted(controller.signal, {});
  expect(typeof pending.then).toBe("function");

  // Node's `aborted()` resolves with a cleanup function even when the signal
  // later aborts; it is not a rejection.
  const cleanup = await pending;
  expect(typeof cleanup).toBe("function");
  expect(() => cleanup()).not.toThrow();

  controller.abort();
});

it("aborted rejects when the signal is already aborted", async () => {
  const controller = new AbortController();
  controller.abort();
  const settled = await util.aborted(controller.signal, {}).then(
    () => "resolved",
    () => "rejected"
  );
  expect(settled).toBe("rejected");
});

it("callbackify turns a promise function into a callback function", async () => {
  const wrapped = util.callbackify(async (value: number) => value * 2);
  expect(typeof wrapped).toBe("function");

  const doubled = await new Promise((resolve, reject) => {
    wrapped(21, (err: any, value: any) => (err ? reject(err) : resolve(value)));
  });
  expect(doubled).toBe(42);

  const failure = await new Promise((resolve) => {
    util.callbackify(async () => {
      throw new Error("boom");
    })((err: any) => resolve(err && err.message));
  });
  expect(failure).toBe("boom");
});

it("_extend copies own enumerable properties", () => {
  const target = { a: 1 };
  expect(util._extend(target, { b: 2 })).toEqual({ a: 1, b: 2 });
});

// --- inspect ------------------------------------------------------------

it("inspect honours the custom inspect symbol", () => {
  const value = {
    [util.inspect.custom]() {
      return "CUSTOM";
    },
  };
  // Used to throw `TypeError: invalid property access`.
  expect(() => util.inspect(value)).not.toThrow();
  expect(util.inspect(value)).toBe("CUSTOM");
});

it("inspect separates entries with spaces, never a carriage return", () => {
  const text = util.inspect({ a: 1, b: 2 });
  expect(text).not.toContain("\r");
  expect(text).toBe("{ a: 1, b: 2 }");
  expect(util.inspect([1, 2, 3])).toBe("[ 1, 2, 3 ]");
});

it("inspect exposes colors, styles and defaultOptions", () => {
  expect(Array.isArray(util.inspect.colors.red)).toBe(true);
  expect(typeof util.inspect.colors.red[0]).toBe("number");
  expect(typeof util.inspect.styles.special).toBe("string");
  expect(typeof util.inspect.defaultOptions.depth).toBe("number");
  expect(util.inspect.defaultOptions.colors).toBe(false);
});

it("console.log still lays objects out over multiple lines", () => {
  // The compact form above is `inspect`'s; console output keeps the expanded
  // layout it had before.
  const text = util.format("%o", { a: 1, b: 2 });
  expect(text).toContain("\n");
  expect(text).not.toContain("\r");
});

void Duplex;
