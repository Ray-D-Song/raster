import defaultImport from "node:inspector";
import legacyImport from "inspector";
import Module from "node:module";

it("node:inspector should be the same as inspector", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

it("url() returns undefined and named/default share the same function", () => {
  expect(typeof defaultImport.url).toBe("function");
  expect(defaultImport.url()).toBeUndefined();
  expect(defaultImport.url).toBe(legacyImport.url);
  expect(defaultImport.default?.url ?? defaultImport.url).toBe(defaultImport.url);
});

it("does not expose Session or open/close protocol APIs", () => {
  expect("Session" in defaultImport).toBe(false);
  expect(defaultImport.Session).toBeUndefined();
  expect(defaultImport.open).toBeUndefined();
  expect(defaultImport.close).toBeUndefined();
  expect(defaultImport.waitForDebugger).toBeUndefined();
});

it("Module.builtinModules includes inspector", () => {
  expect(Module.builtinModules).toContain("inspector");
});
