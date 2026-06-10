import assert from "node:assert/strict";
import { test } from "node:test";

import {
  HOST_EXTERNALS,
  RasterPluginError,
  normalizeRasterOptions,
  validateEsbuildMetafile,
  validateRollupBundle,
} from "../src/core.ts";

test("normalizes default Raster plugin options", () => {
  const options = normalizeRasterOptions();

  assert.equal(options.entry, "src/main.tsx");
  assert.equal(options.outfile, "dist/raster/app.js");
  assert.equal(options.target, "es2022");
  assert.equal(options.sourcemap, false);
  assert.equal(options.minify, true);
  assert.deepEqual(options.external, []);
  assert.deepEqual(options.hostExternal, [...HOST_EXTERNALS]);
  assert.deepEqual(options.allExternal, [...HOST_EXTERNALS]);
});

test("appends user externals without removing host externals", () => {
  const options = normalizeRasterOptions({
    external: ["node:fs", "react", "node:fs"],
  });

  assert.deepEqual(options.external, ["node:fs", "react"]);
  assert.deepEqual(options.allExternal, [...HOST_EXTERNALS, "node:fs"]);
});

test("rejects Rollup-like bundles with multiple JS chunks", () => {
  const options = normalizeRasterOptions({ outfile: "dist/raster/app.js" });

  assert.throws(
    () =>
      validateRollupBundle(
        { file: "dist/raster/app.js" },
        {
          "app.js": { type: "chunk", fileName: "app.js" },
          "lazy.js": { type: "chunk", fileName: "lazy.js" },
        },
        options
      ),
    RasterPluginError
  );
});

test("rejects Rollup-like bundles with assets", () => {
  const options = normalizeRasterOptions({ outfile: "dist/raster/app.js" });

  assert.throws(
    () =>
      validateRollupBundle(
        { file: "dist/raster/app.js" },
        {
          "app.js": { type: "chunk", fileName: "app.js" },
          "style.css": { type: "asset", fileName: "style.css" },
        },
        options
      ),
    RasterPluginError
  );
});

test("rejects Rollup-like bundles with unexpected remaining imports", () => {
  const options = normalizeRasterOptions({ outfile: "dist/raster/app.js" });

  assert.throws(
    () =>
      validateRollupBundle(
        { file: "dist/raster/app.js" },
        {
          "app.js": {
            type: "chunk",
            fileName: "app.js",
            imports: ["react", "raster-js/internal"],
          },
        },
        options
      ),
    RasterPluginError
  );
});

test("validates esbuild metafile output", () => {
  const options = normalizeRasterOptions({ outfile: "dist/raster/app.js" });

  assert.doesNotThrow(() =>
    validateEsbuildMetafile(
      {
        metafile: {
          outputs: {
            "dist/raster/app.js": {
              imports: [{ path: "react", external: true }],
            },
          },
        },
      },
      options
    )
  );
});

test("rejects esbuild metafile output path mismatch", () => {
  const options = normalizeRasterOptions({ outfile: "dist/raster/app.js" });

  assert.throws(
    () =>
      validateEsbuildMetafile(
        {
          metafile: {
            outputs: {
              "dist/raster/other.js": {},
            },
          },
        },
        options
      ),
    RasterPluginError
  );
});
