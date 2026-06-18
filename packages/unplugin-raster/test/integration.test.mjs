import assert from "node:assert/strict";
import { chmod, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";

import { build as esbuild } from "esbuild";
import { build as rolldownBuild } from "rolldown";
import { rollup } from "rollup";
import { build as viteBuild } from "vite";

import esbuildRaster from "../src/esbuild.ts";
import rolldownRaster from "../src/rolldown.ts";
import rollupRaster from "../src/rollup.ts";
import viteRaster from "../src/vite.ts";

process.env.RASTER_UNPLUGIN_SKIP_BINARY = "1";

const EXPECTED_OUTPUTS = ["app.js", "app.js.map"];

test("supported bundlers build a Raster bundle and executable", async (t) => {
  for (const adapter of buildAdapters()) {
    await t.test(adapter.name, async () => {
      await withFixture(async (root) => {
        await writeRasterApp(root);
        const binaryLog = path.join(root, "raster-binary.jsonl");
        const fakeBinary = await writeFakeBinary(root, binaryLog);
        const outfile = path.join(root, "target/raster/app.js");
        const out = path.join(root, "target/raster/app");

        await withEnv(
          {
            RASTER_UNPLUGIN_SKIP_BINARY: undefined,
            RASTER_UNPLUGIN_BINARY: fakeBinary,
          },
          () => adapter.build({ root, outfile, out })
        );

        assert.deepEqual(await outputFiles(root), EXPECTED_OUTPUTS);
        assert.deepEqual((await readJsonLines(binaryLog)).map((call) => call.args), [
          ["build", "--bundle", outfile, "--out", out],
        ]);
      });
    });
  }
});

test("Vite rejects emitted app assets", async () => {
  await withFixture(async (root) => {
    await writeRasterApp(root, {
      extraEntry: 'import "./style.css";',
      files: {
        "src/style.css": "body { color: red; }\n",
      },
    });

    await assert.rejects(
      () =>
        viteBuild({
          root,
          logLevel: "silent",
          plugins: [
            viteRaster({
              entry: path.join(root, "src/main.ts"),
              outfile: path.join(root, "dist/raster/app.js"),
              minify: false,
            }),
          ],
        }),
      /asset output is not supported/
    );
  });
});

function buildAdapters() {
  return [
    {
      name: "Vite",
      build: ({ root }) =>
        viteBuild({
          root,
          logLevel: "silent",
          plugins: [viteRaster({ entry: path.join(root, "src/main.ts") })],
        }),
    },
    {
      name: "esbuild",
      build: ({ root }) =>
        esbuild({
          absWorkingDir: root,
          plugins: [esbuildRaster({ entry: path.join(root, "src/main.ts") })],
        }),
    },
    {
      name: "Rollup",
      build: async ({ root, outfile, out }) => {
        const bundle = await rollup({
          plugins: [rollupRaster({ entry: path.join(root, "src/main.ts"), outfile, out })],
        });
        try {
          await bundle.write({});
        } finally {
          await bundle.close();
        }
      },
    },
    {
      name: "Rolldown",
      build: ({ root, outfile, out }) =>
        rolldownBuild({
          plugins: [rolldownRaster({ entry: path.join(root, "src/main.ts"), outfile, out })],
          output: {},
        }),
    },
  ];
}

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
  await writeFile(path.join(root, "src/lazy.ts"), 'export const label = "loaded";\n');
  for (const [file, source] of Object.entries(options.files ?? {})) {
    await writeFile(path.join(root, file), source);
  }
  await writeFile(
    path.join(root, "src/main.ts"),
    `
import { createRoot } from "raster-js/react";
import { Button } from "raster-js/components";
import { jsx } from "react/jsx-runtime";
import { label } from "./lazy.ts";
${options.extraEntry ?? ""}

async function loadLabel() {
  const mod = await import("./lazy.ts");
  return mod.label;
}

void loadLabel();
createRoot({ width: 320, height: 240 }).render(jsx(Button, { children: label }));
`
  );
}

async function outputFiles(root) {
  return (await readdir(path.join(root, "target/raster"))).sort();
}

async function writeFakeBinary(root, logPath) {
  const binary = path.join(root, process.platform === "win32" ? "fake-raster.cmd" : "fake-raster");
  if (process.platform === "win32") {
    await writeFile(
      binary,
      `@echo off\nnode -e "require('fs').appendFileSync(process.argv[1], JSON.stringify({ args: process.argv.slice(2) }) + '\\n')" "${logPath}" %*\n`
    );
  } else {
    await writeFile(
      binary,
      `#!/usr/bin/env node\nimport fs from "node:fs";\nfs.appendFileSync(${JSON.stringify(
        logPath
      )}, JSON.stringify({ args: process.argv.slice(2) }) + "\\n");\n`
    );
    await chmod(binary, 0o755);
  }
  return binary;
}

async function readJsonLines(file) {
  const source = await readFile(file, "utf8");
  return source
    .trim()
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

async function withEnv(values, run) {
  const previous = new Map();
  for (const key of Object.keys(values)) {
    previous.set(key, process.env[key]);
    if (values[key] === undefined) {
      delete process.env[key];
    } else {
      process.env[key] = values[key];
    }
  }
  try {
    return await run();
  } finally {
    for (const [key, value] of previous) {
      if (value === undefined) {
        delete process.env[key];
      } else {
        process.env[key] = value;
      }
    }
  }
}
