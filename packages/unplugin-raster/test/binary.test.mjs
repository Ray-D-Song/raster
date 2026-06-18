import assert from "node:assert/strict";
import { chmod, mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";

import { startViteBuildWatchForRasterDev, stopRasterDev } from "../src/binary.ts";

test("Vite dev child watch starts once and does not recurse", async () => {
  await withFixture(async (root) => {
    const logPath = path.join(root, "vite.jsonl");
    const fakeVite = await writeFakeBinary(root, logPath);

    await withEnv(
      {
        RASTER_UNPLUGIN_SKIP_BINARY: undefined,
        RASTER_UNPLUGIN_VITE_BIN: fakeVite,
        RASTER_UNPLUGIN_VITE_DEV_CHILD: undefined,
      },
      async () => {
        startViteBuildWatchForRasterDev(rasterOptions(root));
        const calls = await waitForJsonLines(logPath);
        assert.deepEqual(calls.map((call) => call.args), [["build", "--watch"]]);
        assert.equal(calls[0].cwd, await realpath(root));
        assert.equal(calls[0].env.RASTER_UNPLUGIN_VITE_DEV_CHILD, "1");
        stopRasterDev();
      }
    );

    await rm(logPath, { force: true });
    await withEnv(
      {
        RASTER_UNPLUGIN_SKIP_BINARY: undefined,
        RASTER_UNPLUGIN_VITE_BIN: fakeVite,
        RASTER_UNPLUGIN_VITE_DEV_CHILD: "1",
      },
      async () => {
        startViteBuildWatchForRasterDev(rasterOptions(root));
        await delay(100);
        await assert.rejects(() => readFile(logPath, "utf8"), { code: "ENOENT" });
      }
    );
  });
});

async function withFixture(run) {
  const root = await mkdtemp(path.join(os.tmpdir(), "raster-binary-"));
  try {
    await run(root);
  } finally {
    stopRasterDev();
    await rm(root, { recursive: true, force: true });
  }
}

function rasterOptions(root) {
  return {
    entry: path.join(root, "src/main.ts"),
    outfile: path.join(root, "target/raster/app.js"),
    out: path.join(root, "target/raster/app"),
    root,
    watch: false,
    target: "es2022",
    sourcemap: true,
    minify: false,
    external: [],
    hostExternal: [],
    allExternal: [],
  };
}

async function writeFakeBinary(root, logPath) {
  const binary = path.join(root, process.platform === "win32" ? "fake-vite.cmd" : "fake-vite");
  if (process.platform === "win32") {
    await writeFile(
      binary,
      `@echo off\nnode -e "require('fs').appendFileSync(process.argv[1], JSON.stringify({ args: process.argv.slice(2), cwd: process.cwd(), env: process.env }) + '\\n')" "${logPath}" %*\n`
    );
  } else {
    await writeFile(
      binary,
      `#!/usr/bin/env node\nimport fs from "node:fs";\nfs.appendFileSync(${JSON.stringify(
        logPath
      )}, JSON.stringify({ args: process.argv.slice(2), cwd: process.cwd(), env: process.env }) + "\\n");\n`
    );
    await chmod(binary, 0o755);
  }
  return binary;
}

async function waitForJsonLines(file) {
  const started = Date.now();
  while (Date.now() - started < 5000) {
    try {
      const source = await readFile(file, "utf8");
      const lines = source.trim().split(/\r?\n/).filter(Boolean);
      if (lines.length > 0) {
        return lines.map((line) => JSON.parse(line));
      }
    } catch (error) {
      if (error?.code !== "ENOENT") {
        throw error;
      }
    }
    await delay(25);
  }
  throw new Error(`Timed out waiting for ${file}`);
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
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
