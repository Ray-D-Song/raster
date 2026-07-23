import defaultImport from "node:stream";
import legacyImport from "stream";
import Module from "node:module";

const _require = require;

it("node:stream should be the same as stream", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

it("require('stream') matches the ESM default export", () => {
  const required = _require("stream");
  expect(required).toStrictEqual(legacyImport);
});

it("require('node:stream') shares the same cache object as bare require", () => {
  expect(_require("node:stream")).toBe(_require("stream"));
});

it("require('stream/promises') and node: alias share one cache object", () => {
  const bare = _require("stream/promises");
  const aliased = _require("node:stream/promises");
  expect(bare).toBe(aliased);
  expect(bare).toBeDefined();
});

it("Module.isBuiltin recognizes stream and stream/promises", () => {
  expect(Module.isBuiltin("stream")).toBe(true);
  expect(Module.isBuiltin("node:stream")).toBe(true);
  expect(Module.isBuiltin("stream/promises")).toBe(true);
  expect(Module.isBuiltin("node:stream/promises")).toBe(true);
});

it("Module.builtinModules lists public embedded stream modules only", () => {
  expect(Module.builtinModules).toContain("stream");
  expect(Module.builtinModules).toContain("stream/promises");
  expect(Module.builtinModules).not.toContain("raster_runtime:test/index");
});

import {
  TextEncoderStream,
  TextDecoderStream,
  ReadableStream,
  WritableStream,
} from "node:stream/web";

it("exposes encoding streams on globals and stream/web", () => {
  expect(globalThis.TextEncoderStream).toBe(TextEncoderStream);
  expect(globalThis.TextDecoderStream).toBe(TextDecoderStream);
  expect(new TextEncoderStream().encoding).toBe("utf-8");
});

it("uses Raster stream instances for encoding stream sides", () => {
  const encoder = new TextEncoderStream();
  expect(encoder.readable).toBeInstanceOf(ReadableStream);
  expect(encoder.writable).toBeInstanceOf(WritableStream);
  expect(new TextDecoderStream().encoding).toBe("utf-8");
});
