import assert from "node:assert/strict";
import { chmod, mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";

import viteRaster from "../src/vite.ts";
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
        RASTER_UNPLUGIN_TEST_DISABLE_PARENT_EXIT: "1",
      },
      async () => {
        startViteBuildWatchForRasterDev(rasterOptions(root));
        const calls = await waitForJsonLines(logPath);
        assert.deepEqual(calls.map((call) => call.args), [["build", "--watch"]]);
        assert.equal(calls[0].cwd, await realpath(root));
        assert.equal(calls[0].env.RASTER_UNPLUGIN_VITE_DEV_CHILD, "1");
        assert.equal(calls[0].env.NODE_ENV, "production");
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

test("Vite dev serve mode does not listen with the native server", async () => {
  await withFixture(async (root) => {
    const logPath = path.join(root, "vite.jsonl");
    const fakeVite = await writeFakeBinary(root, logPath, { stayAlive: true });
    const plugin = viteRaster({ entry: path.join(root, "src/main.ts") });
    let nativeListenCalled = false;
    let nativeCloseCalled = false;
    let nativePrintUrlsCalled = false;

    await withEnv(
      {
        RASTER_UNPLUGIN_SKIP_BINARY: undefined,
        RASTER_UNPLUGIN_VITE_BIN: fakeVite,
        RASTER_UNPLUGIN_VITE_DEV_CHILD: undefined,
        RASTER_UNPLUGIN_TEST_DISABLE_PARENT_EXIT: "1",
      },
      async () => {
        const server = {
          listen: async () => {
            nativeListenCalled = true;
          },
          close: async () => {
            nativeCloseCalled = true;
          },
          printUrls: () => {
            nativePrintUrlsCalled = true;
          },
        };

        plugin.config?.({ root }, { command: "serve", mode: "development" });
        plugin.configureServer?.(server);

        const result = await server.listen();
        assert.equal(nativeListenCalled, false);
        assert.equal(nativeCloseCalled, false);
        server.printUrls();
        assert.equal(nativePrintUrlsCalled, false);
        assert.equal(result, server);
        await waitForJsonLines(logPath);
        await server.close();
        assert.equal(nativeCloseCalled, true);
      }
    );
  });
});

test("Vite dev serve mode closes when the child watch exits", async () => {
  await withFixture(async (root) => {
    const logPath = path.join(root, "vite.jsonl");
    const fakeVite = await writeFakeBinary(root, logPath);
    const plugin = viteRaster({ entry: path.join(root, "src/main.ts") });
    let nativeCloseCalled = false;

    await withEnv(
      {
        RASTER_UNPLUGIN_SKIP_BINARY: undefined,
        RASTER_UNPLUGIN_VITE_BIN: fakeVite,
        RASTER_UNPLUGIN_VITE_DEV_CHILD: undefined,
        RASTER_UNPLUGIN_TEST_DISABLE_PARENT_EXIT: "1",
      },
      async () => {
        const server = {
          listen: async () => server,
          close: async () => {
            nativeCloseCalled = true;
          },
          printUrls: () => {},
        };

        plugin.config?.({ root }, { command: "serve", mode: "development" });
        plugin.configureServer?.(server);
        await waitForJsonLines(logPath);
        await waitFor(() => nativeCloseCalled);
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
    allExternal: [],
  };
}

async function writeFakeBinary(root, logPath, options = {}) {
  const binary = path.join(root, process.platform === "win32" ? "fake-vite.cmd" : "fake-vite");
  if (process.platform === "win32") {
    await writeFile(
      binary,
      `@echo off\nnode -e "require('fs').appendFileSync(process.argv[1], JSON.stringify({ args: process.argv.slice(2), cwd: process.cwd(), env: process.env }) + '\\n'); ${options.stayAlive ? "setInterval(() => {}, 1000)" : ""}" "${logPath}" %*\n`
    );
  } else {
    await writeFile(
      binary,
      `#!/usr/bin/env node\nimport fs from "node:fs";\nfs.appendFileSync(${JSON.stringify(
        logPath
      )}, JSON.stringify({ args: process.argv.slice(2), cwd: process.cwd(), env: process.env }) + "\\n");\n${
        options.stayAlive ? "setInterval(() => {}, 1000);\n" : ""
      }`
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

async function waitFor(predicate) {
  const started = Date.now();
  while (Date.now() - started < 5000) {
    if (predicate()) {
      return;
    }
    await delay(25);
  }
  throw new Error("Timed out waiting for condition");
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
