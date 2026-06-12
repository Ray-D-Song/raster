import assert from "node:assert/strict";
import { mkdtemp, readdir, rm, writeFile, mkdir } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";

import { build as esbuild } from "esbuild";
import { build as rolldownBuild } from "rolldown";
import { build as viteBuild } from "vite";

import esbuildRaster from "../src/esbuild.ts";
import rolldownRaster from "../src/rolldown.ts";
import viteRaster from "../src/vite.ts";

process.env.RASTER_UNPLUGIN_SKIP_BINARY = "1";

test("Vite builds a single Raster app bundle", async () => {
  await withFixture(async (root) => {
    await writeRasterApp(root);
    const outfile = path.join(root, "dist/raster/app.js");

    await viteBuild({
      root,
      logLevel: "silent",
      plugins: [
        viteRaster({
          entry: path.join(root, "src/main.ts"),
          outfile,
          minify: false,
        }),
      ],
    });

    assert.deepEqual(await outputFiles(root), ["app.js"]);
  });
});

test("esbuild builds a single Raster app bundle", async () => {
  await withFixture(async (root) => {
    await writeRasterApp(root);
    const outfile = path.join(root, "dist/raster/app.js");

    await esbuild({
      absWorkingDir: root,
      plugins: [
        esbuildRaster({
          entry: path.join(root, "src/main.ts"),
          outfile,
          minify: false,
        }),
      ],
    });

    assert.deepEqual(await outputFiles(root), ["app.js"]);
  });
});

test("Rolldown builds a single Raster app bundle", async () => {
  await withFixture(async (root) => {
    await writeRasterApp(root);
    const outfile = path.join(root, "dist/raster/app.js");

    await rolldownBuild({
      plugins: [
        rolldownRaster({
          entry: path.join(root, "src/main.ts"),
          outfile,
          minify: false,
        }),
      ],
      output: {},
    });

    assert.deepEqual(await outputFiles(root), ["app.js"]);
  });
});

test("Vite rejects emitted app assets", async () => {
  await withFixture(async (root) => {
    await writeRasterApp(root, {
      extraEntry: 'import "./style.css";',
      files: {
        "src/style.css": "body { color: red; }\n",
      },
    });
    const outfile = path.join(root, "dist/raster/app.js");

    await assert.rejects(
      () =>
        viteBuild({
          root,
          logLevel: "silent",
          plugins: [
            viteRaster({
              entry: path.join(root, "src/main.ts"),
              outfile,
              minify: false,
            }),
          ],
        }),
      /asset output is not supported/
    );
  });
});

async function withFixture(run) {
  const root = await mkdtemp(path.join(os.tmpdir(), "raster-plugin-"));
  try {
    await mkdir(path.join(root, "src"), { recursive: true });
    await writeFile(path.join(root, "package.json"), '{"type":"module"}\n');
    await run(root);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

async function writeRasterApp(root, options = {}) {
  await writeFile(
    path.join(root, "src/lazy.ts"),
    'export const label = "loaded";\n'
  );
  for (const [file, source] of Object.entries(options.files ?? {})) {
    await writeFile(path.join(root, file), source);
  }
  await writeFile(
    path.join(root, "src/main.ts"),
    `
import { createRoot } from "raster-js/react";
import { jsx } from "react/jsx-runtime";
import { label } from "./lazy.ts";
${options.extraEntry ?? ""}

async function loadLabel() {
  const mod = await import("./lazy.ts");
  return mod.label;
}

void loadLabel();
createRoot({ width: 320, height: 240 }).render(jsx("View", { children: label }));
`
  );
}

async function outputFiles(root) {
  return (await readdir(path.join(root, "dist/raster"))).sort();
}
