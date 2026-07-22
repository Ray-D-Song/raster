// Raster-only: the `.test.raster.ts` suffix keeps host Vitest from collecting this fixture.
const CWD = process.cwd();
const _require = require;

const Module = _require("node:module");
const legacyModule = _require("module");

it("node:module should be the same as module", () => {
  expect(Module).toStrictEqual(legacyModule);
});

it("default export should be a callable Module constructor", () => {
  expect(typeof Module).toBe("function");
  expect(typeof legacyModule).toBe("function");
  expect(Module.prototype.require).toBeInstanceOf(Function);
});

const { createRequire, _resolveFilename, _nodeModulePaths, _cache, builtinModules, isBuiltin } =
  Module;

it("should resolve 'node:module via createRequire()", () => {
  const __require = createRequire(import.meta.url);
  expect(__require("node:module").createRequire).toBeDefined();
});

it("createRequire returns a require function with resolve and cache", () => {
  const localRequire = createRequire(import.meta.url);
  expect(typeof localRequire).toBe("function");
  expect(typeof localRequire.resolve).toBe("function");
  expect(localRequire.cache).toBe(_cache);
  expect(_require.cache).toBe(_cache);
});

it("createRequire accepts URL objects", () => {
  const localRequire = createRequire(new URL(import.meta.url));
  expect(localRequire.resolve("node:path")).toBe("node:path");
});

it("require.resolve resolves builtins, relative fixtures, and package entries", () => {
  expect(_require.resolve("node:path")).toBe("node:path");
  expect(_require.resolve(`${CWD}/fixtures/hello.js`)).toContain("fixtures/hello.js");
  expect(_require.resolve("package.json")).toContain("package.json");
});

it("Module._resolveFilename resolves builtins and relative fixtures", () => {
  expect(_resolveFilename("fs")).toBe("fs");
  expect(_resolveFilename("node:fs")).toBe("fs");
  expect(_resolveFilename(`${CWD}/fixtures/hello.js`, null, false, {})).toContain(
    "fixtures/hello.js",
  );
});

it("Module._nodeModulePaths returns ordered node_modules directories", () => {
  const paths = _nodeModulePaths(`${CWD}/a/b/c`);
  expect(paths.length).toBeGreaterThan(0);
  expect(paths[0]).toContain("/a/b/c/node_modules");
  expect(paths.at(-1)).toContain("/node_modules");
});

it("Module._nodeModulePaths skips nested node_modules segments", () => {
  const paths = _nodeModulePaths("/a/node_modules");
  expect(paths).not.toContain("/a/node_modules/node_modules");
  expect(paths).toContain("/a/node_modules");
});

it("Module.prototype.require.call resolves relative fixtures", () => {
  const mod = new Module(`${CWD}/fixtures/hello.js`);
  const hello = Module.prototype.require.call(mod, "./hello.js");
  expect(hello.hello).toBe("hello world!");
});

it("handles cyclic requires", () => {
  const a = _require(`${CWD}/fixtures/a.js`);
  const b = _require(`${CWD}/fixtures/b.js`);
  expect(a.done).toEqual(b.done);
});

it("deleting require.cache entry reloads the module", () => {
  const target = _require.resolve(`${CWD}/fixtures/hello.js`);
  const first = _require(`${CWD}/fixtures/hello.js`);
  delete _require.cache[target];
  const second = _require(`${CWD}/fixtures/hello.js`);
  expect(first).not.toBe(second);
  expect(first.hello).toBe(second.hello);
});

it("deleting require.cache entry reloads json modules", () => {
  const target = _require.resolve(`${CWD}/fixtures/package.json`);
  const first = _require(`${CWD}/fixtures/package.json`);
  delete _require.cache[target];
  const second = _require(`${CWD}/fixtures/package.json`);
  expect(first).not.toBe(second);
  expect(first.private).toBe(second.private);
});

it("module records link parent and children", () => {
  const aFile = _require.resolve(`${CWD}/fixtures/a.js`);
  _require(`${CWD}/fixtures/a.js`);
  const aRecord = _require.cache[aFile];
  const bFile = _require.resolve(`${CWD}/fixtures/b.js`);
  const bRecord = _require.cache[bFile];
  expect(aRecord.parent).toBeNull();
  expect(bRecord.parent).toBe(aRecord);
  expect(aRecord.children).toContain(bRecord);
});

it("overwriting _resolveFilename affects subsequent require calls", () => {
  const original = _resolveFilename;
  const aliasTarget = `${CWD}/fixtures/hello.js`;
  try {
    Module._resolveFilename = (request, parent, isMain, options) => {
      if (request === "aliased-fixture") {
        return aliasTarget;
      }
      return original.call(Module, request, parent, isMain, options);
    };
    const mod = { filename: `${CWD}/fixtures/hello.js`, paths: _nodeModulePaths(CWD) };
    expect(Module.prototype.require.call(mod, "aliased-fixture").hello).toBe("hello world!");
  } finally {
    Module._resolveFilename = original;
  }
});

it("isBuiltin recognizes raster builtins", () => {
  expect(isBuiltin("path")).toBe(true);
  expect(isBuiltin("node:path")).toBe(true);
  expect(isBuiltin("not-a-real-builtin-module")).toBe(false);
});

it("builtinModules is populated", () => {
  expect(Array.isArray(builtinModules)).toBe(true);
  expect(builtinModules).toContain("path");
});
