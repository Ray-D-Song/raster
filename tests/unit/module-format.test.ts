const CWD = process.cwd();
import { spawnCapture } from "./test-utils";

const FIXTURES = `${CWD}/fixtures/module-format`;

describe("module format detection", () => {
  it("runs typeless extensionless CommonJS as a CLI entry", async () => {
    const { code, stdout, stderr } = await spawnCapture(process.argv0, [
      `${FIXTURES}/typeless/cli`,
    ]);
    expect({ code, stdout, stderr }).toEqual({
      code: 0,
      stdout: "typeless-cli:cjs\n",
      stderr: "",
    });
  });

  it("imports typeless .js ESM syntax through CJS-first fallback", async () => {
    const mod = await import(`${FIXTURES}/typeless/esm-syntax.js`);
    expect(mod.label).toBe("esm-fallback");
    expect(mod.default).toBe("esm-fallback");
  });

  it("loads type:commonjs .js through the CJS facade", async () => {
    const mod = await import(`${FIXTURES}/type-commonjs/mod.js`);
    expect(mod.default).toEqual({ label: "type-commonjs-js" });
    expect(mod.label).toBe("type-commonjs-js");
  });

  it("keeps type:module .js and extensionless files as ESM", async () => {
    const js = await import(`${FIXTURES}/type-module/mod.js`);
    const ext = await import(`${FIXTURES}/type-module/cli`);
    expect(js.label).toBe("type-module-js");
    expect(js.default).toBe("type-module-js");
    expect(ext.label).toBe("type-module-ext");
    expect(ext.default).toBe("type-module-ext");
  });

  it("lets .cjs and .mjs override package type", async () => {
    const cjs = await import(`${FIXTURES}/overrides/forced.cjs`);
    const mjs = await import(`${FIXTURES}/overrides/forced.mjs`);
    expect(cjs.default).toEqual({ label: "always-cjs" });
    expect(cjs.label).toBe("always-cjs");
    expect(mjs.label).toBe("always-esm");
    expect(mjs.default).toBe("always-esm");
  });

  it("does not inherit project root type across node_modules boundary", async () => {
    const { code, stdout, stderr } = await spawnCapture(process.argv0, [
      `${FIXTURES}/project-root/node_modules/orphan-dep/bin/cli`,
    ]);
    expect({ code, stdout, stderr }).toEqual({
      code: 0,
      stdout: "orphan:orphan-cjs\n",
      stderr: "",
    });
  });

  it("rejects ESM syntax in type:commonjs packages instead of falling back", async () => {
    const { code, stdout, stderr } = await spawnCapture(process.argv0, [
      `${FIXTURES}/type-commonjs-esm/module.js`,
    ]);
    expect(code).not.toBe(0);
    expect(stdout).not.toContain("executed");
    expect(stderr.length).toBeGreaterThan(0);
  });

  it("fails when the nearest package.json is invalid JSON", async () => {
    const { code, stderr } = await spawnCapture(process.argv0, [
      `${FIXTURES}/invalid-package-json/mod.js`,
    ]);
    expect(code).not.toBe(0);
    expect(stderr).toContain("Invalid package config");
  });

  it("propagates invalid package config when importing a package by name", async () => {
    const { code, stderr } = await spawnCapture(process.argv0, [
      `${FIXTURES}/import-invalid-pjson/entry.mjs`,
    ]);
    expect(code).not.toBe(0);
    expect(stderr).toContain("Invalid package config");
    expect(stderr).not.toContain("Error resolving module 'invalid-pjson'");
  });

  it("rejects non-string package.json type as invalid package config", async () => {
    const { code, stderr } = await spawnCapture(process.argv0, [
      `${FIXTURES}/type-non-string/mod.js`,
    ]);
    expect(code).not.toBe(0);
    expect(stderr).toContain("Invalid package config");
  });

  it("lets require() load explicit .mjs through the ESM loader", async () => {
    const { code, stdout, stderr } = await spawnCapture(process.argv0, [
      "-e",
      `require("${FIXTURES}/overrides/forced.mjs"); console.log("ok")`,
    ]);
    expect({ code, stdout, stderr }).toEqual({
      code: 0,
      stdout: "ok\n",
      stderr: "",
    });
  });

  it("continues node_modules search when earlier dirs lack the package", async () => {
    const { code, stdout, stderr } = await spawnCapture(process.argv0, [
      `${FIXTURES}/search-order/a/b/entry.js`,
    ]);
    expect({ code, stdout, stderr }).toEqual({
      code: 0,
      stdout: "later-pkg\n",
      stderr: "",
    });
  });

  it("resolves nested package subpaths without exports via directory main", async () => {
    const { code, stdout, stderr } = await spawnCapture(process.argv0, [
      `${FIXTURES}/nested-no-exports/entry.js`,
    ]);
    expect({ code, stdout, stderr }).toEqual({
      code: 0,
      stdout: "commander-main\n",
      stderr: "",
    });
  });
});
