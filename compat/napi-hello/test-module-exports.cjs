"use strict";

const path = require("node:path");

const addonPath = path.join(__dirname, "build", "Release", "exports_replace.node");

function assert(condition, message) {
  if (!condition) {
    console.error(message);
    process.exit(1);
  }
}

const moduleObj = { exports: { original: true } };
const before = moduleObj.exports;
process.dlopen(moduleObj, addonPath);
const loaded = moduleObj.exports;

assert(loaded !== before, "module.exports should be replaced, not mutated in place");
assert(loaded.__identity === "replacement-exports-v1", "__identity marker");
assert(loaded.value === 42, "primitive export");
assert(loaded.replFn() === "fn-ok", "function export");
assert(loaded.instance && loaded.instance.kind === "instance", "object instance export");
assert(loaded.fromGetter === "getter-ok", "getter export");

const symbols = Object.getOwnPropertySymbols(loaded);
assert(symbols.length === 1, "expected one symbol export");
assert(loaded[symbols[0]] === "symval", "symbol export value");

const required = require(addonPath);
assert(required.__identity === "replacement-exports-v1", "require() sees replacement exports");
assert(required === loaded || required.__identity === loaded.__identity, "require identity");

const primitivePath = path.join(__dirname, "build", "Release", "exports_primitive.node");
const primitiveModule = { exports: { original: true } };
process.dlopen(primitiveModule, primitivePath);
assert(primitiveModule.exports === "primitive-export", "primitive module.exports replacement");

console.log("module-exports-replace-ok");
