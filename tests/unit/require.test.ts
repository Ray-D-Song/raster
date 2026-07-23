const _require = require; //used to preserve require during bundling/minification
const CWD = process.cwd();
import { spawn } from "node:child_process";
import { spawnCapture } from "./test-utils";

import { platform } from "node:os";
const IS_WINDOWS = platform() === "win32";

it("should require a file (absolute path)", () => {
  const { hello } = _require(`${CWD}/fixtures/hello.js`);

  expect(hello).toEqual("hello world!");
});

it("should require a json file (absolute path)", () => {
  const a = _require(`${CWD}/package.json`);

  expect(a.private).toEqual(true);
});

it("should require a js file (relative path)", () => {
  const { hello } = _require("../../../../fixtures/hello.js");

  expect(hello).toEqual("hello world!");
});

it("should require a json file (relative path)", () => {
  const a = _require("../../../../fixtures/package.json");

  expect(a.private).toEqual(true);
});

it("should require a json file (path unspecified)", () => {
  const a = _require("package.json");

  expect(a.private).toEqual(true);
});

it("should require a file (file schema)", () => {
  const { hello } = _require(`file://${CWD}/fixtures/hello.js`);

  expect(hello).toEqual("hello world!");
});

it("should require a json file (file schema)", () => {
  const a = _require(`file://${CWD}/package.json`);

  expect(a.private).toEqual(true);
});

it("should return same module when require multiple files", () => {
  const { hello: hello1 } = _require(`${CWD}/fixtures/hello.js`);
  const { hello: hello2 } = _require(`${CWD}/fixtures/hello.js`);
  const { hello: hello3 } = _require(`${CWD}/fixtures/hello.js`);

  expect(hello1).toEqual(hello2);
  expect(hello1).toEqual(hello3);
});

it("should handle cyclic requires", () => {
  const a = _require(`${CWD}/fixtures/a.js`);
  const b = _require(`${CWD}/fixtures/b.js`);

  expect(a.done).toEqual(b.done);
});

it("should provide module-local __filename and __dirname for CommonJS modules", () => {
  const parentPath = `${CWD}/fixtures/cjs-dirname-parent.cjs`;
  const childPath = `${CWD}/fixtures/cjs-dirname-child/child.cjs`;
  const normalize = (value: string) => value.replace(/\\/g, "/");

  const parent = _require(parentPath);

  expect(normalize(parent.__filename)).toBe(normalize(parentPath));
  expect(normalize(parent.__dirname)).toBe(normalize(`${CWD}/fixtures`));
  expect(normalize(parent.__dirname)).toBe(normalize(parent.__filename.replace(/\/[^/]+$/, "")));

  expect(normalize(parent.child.__filename)).toBe(normalize(childPath));
  expect(normalize(parent.child.__dirname)).toBe(normalize(`${CWD}/fixtures/cjs-dirname-child`));
  expect(parent.child.__filename).not.toBe(parent.__filename);
  expect(parent.child.__dirname).not.toBe(parent.__dirname);
});

it("should allow reassignment of module-local __filename and __dirname", () => {
  const normalize = (value: string) => value.replace(/\\/g, "/");
  const modulePath = `${CWD}/fixtures/cjs-dirname-reassign.cjs`;
  const result = _require(modulePath);

  expect(normalize(result.originalDirname)).toBe(normalize(`${CWD}/fixtures`));
  expect(normalize(result.originalFilename)).toBe(normalize(modulePath));
  expect(result.mutatedDirname).toBe("/mutated-dirname");
  expect(result.mutatedFilename).toBe("/mutated-filename");
});

it("should handle cjs requires", () => {
  const a = _require(`${CWD}/fixtures/import.cjs`);

  expect(a.c).toEqual("c");
});

it("should handle cjs requires", () => {
  const a = _require(`${CWD}/fixtures/prop-export.cjs`);

  expect(a.prop).toEqual("a");
});

it("should be able to use node module with prefix `node:` with require", () => {
  let { Console } = require("node:console");
  const consoleObj = new Console({
    stdout: process.stdout,
    stderr: process.stderr,
  });

  // we check if the log does not throw an exception when called
  consoleObj.log("log");
  consoleObj.debug("debug");
  consoleObj.info("info");
  consoleObj.assert(false, "text for assertion should display");
  consoleObj.assert(true, "This text should not be seen");

  consoleObj.warn("warn");
  consoleObj.error("error");
  consoleObj.trace("trace");
});

it("should be able to import exported functions", () => {
  const importedFunction = _require(`${CWD}/fixtures/export-function.cjs`);
  expect(importedFunction()).toBe("hello world!");
});

it("should return same value for multiple require statements", () => {
  const filename = `${CWD}/fixtures/prop-export.cjs`;
  const a = _require(filename);
  const b = _require(filename);
  expect(a).toStrictEqual(b);
});

it("should return all props", () => {
  const a = _require(`${CWD}/fixtures/define-property-export.cjs`);
  expect(a.__esModule).toBe(true);
});

it("should import cjs modules using import statement", async () => {
  const filename = `${CWD}/fixtures/prop-export.cjs`;
  const a = await import(filename);
  const b = await import(filename);
  const c = _require(filename);
  expect(a).toStrictEqual(b);
  expect(a.default).toStrictEqual(c);
  expect(b.default).toStrictEqual(c);
});

it("should handle inner referenced exports", () => {
  const a = _require(`${CWD}/fixtures/referenced-exports.cjs`);
  expect(a.cat()).toBe("str");
  expect(a.length()).toBe(1);
});

if (!IS_WINDOWS) {
  it("should handle named exports from CJS imports", (cb) => {
    spawn(process.argv0, [
      "-e",
      `import {cat} from "${CWD}/fixtures/referenced-exports.cjs"`,
    ]).on("close", (code) => {
      expect(code).toBe(0);
      cb();
    });
  });
}

it("require builtin modules", () => {
  _require("path");
});

it("require `debug` module element", () => {
  _require(`${CWD}/fixtures/test_modules/test-debug.js`);
});

it("require `lodash.merge` module element", () => {
  _require(`${CWD}/fixtures/test_modules/test-lodash.merge.js`);
});

it("require `uuid` module element", () => {
  _require(`${CWD}/fixtures/test_modules/test-uuid.js`);
});

it("require `react-dom` module element", () => {
  _require(`${CWD}/fixtures/test_modules/test-react-dom.js`);
});

it("require `hono/utils/url` module element", () => {
  _require(`${CWD}/fixtures/test_modules/test-elem-hono.js`);
});

it("regression testing for issue #903", () => {
  expect(() => _require(`${CWD}/fixtures/test903/foo.mjs`)).toThrow(
    /Error resolving module /
  );
});

it("regression testing for issue #1245", () => {
  _require(`${CWD}/fixtures/test1245/main/foo.js`);
});

describe("require() CJS/ESM default interop", () => {
  const dir = `${CWD}/fixtures/require-default-interop`;

  it("does not depend on user-overridable Object.isExtensible", () => {
    const original = Object.isExtensible;
    try {
      // @ts-expect-error intentional polyfill/user override
      Object.isExtensible = undefined;
      expect(() => _require("v8")).not.toThrow();
      expect(typeof _require("v8").getHeapStatistics).toBe("function");
    } finally {
      Object.isExtensible = original;
    }
  });

  it("keeps .default and named exports for ordinary user ESM (namespace interop)", () => {
    const mod = _require(`${dir}/extensible.mjs`);
    expect(mod.__esModule).toBe(true);
    expect(mod.default.base).toBe(true);
    expect(mod.named).toBe(42);
    // Default export object is not itself mutated with a self-ref.
    expect(Object.prototype.hasOwnProperty.call(mod.default, "default")).toBe(
      false
    );
  });

  it("preserves frozen default export objects on user ESM without merge failure", () => {
    const complete = _require(`${dir}/frozen-complete.mjs`);
    expect(complete.__esModule).toBe(true);
    expect(Object.isFrozen(complete.default)).toBe(true);
    expect(complete.default.named).toBe(1);
    expect(complete.named).toBe(1);
    expect(complete.extra).toBe(2);

    // Named export is not on the frozen empty default; namespace still exposes both.
    const incomplete = _require(`${dir}/frozen-incomplete.mjs`);
    expect(Object.isFrozen(incomplete.default)).toBe(true);
    expect(incomplete.named).toBe(42);
    expect(incomplete.default.named).toBeUndefined();
  });

  it("preserves sealed/preventExtensions defaults via namespace interop", () => {
    const sealed = _require(`${dir}/sealed-complete.mjs`);
    expect(Object.isSealed(sealed.default)).toBe(true);
    expect(sealed.default.named).toBe(7);
    expect(sealed.named).toBe(7);

    const pe = _require(`${dir}/prevent-extensions-complete.mjs`);
    expect(Object.isExtensible(pe.default)).toBe(false);
    expect(pe.default.named).toBe(9);
    expect(pe.named).toBe(9);
  });

  it("does not add __esModule for named-only ESM (Node-compatible)", () => {
    const mod = _require(`${dir}/named-only.mjs`);
    expect(typeof mod.handler).toBe("function");
    expect("__esModule" in mod).toBe(false);
    expect(Object.keys(mod)).not.toContain("__esModule");
    expect(mod.default).toBeUndefined();

    // Existing named-only fixture used by other suites.
    const primitive = _require(`${CWD}/fixtures/primitive-handler.mjs`);
    expect(typeof primitive.handler).toBe("function");
    expect("__esModule" in primitive).toBe(false);
    expect(Object.keys(primitive)).not.toContain("__esModule");
  });
});

//create a test that spawns a subprocess and executes require.mjs from fixtures and captures stdout
it("should handle blocking requires", async () => {
  const { code, stdout } = await spawnCapture(process.argv0, [
    `${CWD}/fixtures/require.mjs`,
  ]);
  expect(code).toBe(0);
  expect(stdout).toBe(
    ["1", "2", "3", "4", "5", "hello world!", "6", ""].join("\n")
  );
});
