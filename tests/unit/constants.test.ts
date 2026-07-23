import Module from "node:module";
import { createRequire } from "node:module";
import constants from "node:constants";
import legacy from "constants";
import { F_OK, R_OK, W_OK, X_OK } from "node:constants";
import fs from "node:fs";

const require = createRequire(import.meta.url);

it("node:constants should match constants default", () => {
  expect(constants.F_OK).toBe(legacy.F_OK);
  expect(constants.R_OK).toBe(legacy.R_OK);
  expect(constants.W_OK).toBe(legacy.W_OK);
  expect(constants.X_OK).toBe(legacy.X_OK);
});

it("Module.isBuiltin recognizes constants", () => {
  expect(Module.isBuiltin("constants")).toBe(true);
  expect(Module.isBuiltin("node:constants")).toBe(true);
  expect(Module.builtinModules).toContain("constants");
});

it("matches fs.constants values and shape", () => {
  expect(F_OK).toBe(0);
  expect(R_OK).toBe(4);
  expect(W_OK).toBe(2);
  expect(X_OK).toBe(1);
  expect(constants.F_OK).toBe(fs.constants.F_OK);
  expect(constants.R_OK).toBe(fs.constants.R_OK);
  expect(constants.W_OK).toBe(fs.constants.W_OK);
  expect(constants.X_OK).toBe(fs.constants.X_OK);
});

it("require('constants') is frozen flat object without O_SYMLINK", () => {
  const c = require("constants");
  expect(c.F_OK).toBe(fs.constants.F_OK);
  expect(Object.isFrozen(c)).toBe(true);
  expect("O_SYMLINK" in c).toBe(false);
  expect(c.default).toBeUndefined();
  expect(Object.prototype.hasOwnProperty.call(c, "default")).toBe(false);
  expect(Object.prototype.hasOwnProperty.call(c, "__esModule")).toBe(false);
  try {
    c.F_OK = 99;
  } catch {
    // freeze may throw
  }
  expect(c.F_OK).toBe(0);

  const nodeC = require("node:constants");
  expect(nodeC.F_OK).toBe(0);
});

it("require('constants') does not depend on user-overridable Object.isExtensible", () => {
  const original = Object.isExtensible;
  try {
    // @ts-expect-error intentional polyfill/user override
    Object.isExtensible = undefined;
    expect(() => require("constants")).not.toThrow();
    expect(require("constants").F_OK).toBe(0);
  } finally {
    Object.isExtensible = original;
  }
});
