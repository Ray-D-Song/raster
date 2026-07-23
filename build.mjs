import * as esbuild from "esbuild";
import fs from "node:fs/promises";
import path from "node:path";

process.env.NODE_PATH = ".";

const SRC_DIR = path.join("raster_runtime_core", "src", "modules", "js");
const TESTS_DIR = "tests";
const TESTS_SUB_DIR = process.env.TEST_SUB_DIR || "unit";
const OUT_DIR = "bundle/js";
const MINIFY_JS = process.env.JS_MINIFY !== "0";
const ENTRYPOINTS = [
  "stream",
  "stream/promises",
  "@raster_runtime/test/index",
  "@raster_runtime/test/worker",
];

async function readFilesRecursive(dir, filePredicate) {
  const dirents = await fs.readdir(dir, { withFileTypes: true });
  const files = await Promise.all(
    dirents.map((dirent) => {
      const filePath = path.join(dir, dirent.name);

      if (dirent.isDirectory()) {
        return readFilesRecursive(filePath, filePredicate);
      }
      return filePredicate(filePath) ? filePath : [];
    })
  );
  return Array.prototype.concat(...files);
}

const TEST_FILES = await readFilesRecursive(
  path.join(TESTS_DIR, TESTS_SUB_DIR),
  (filePath) =>
    filePath.endsWith(".test.ts") ||
    filePath.endsWith(".test.raster.ts") ||
    filePath.endsWith(".spec.ts") ||
    filePath.endsWith(".any.js")
);

const ES_BUILD_OPTIONS = {
  splitting: MINIFY_JS,
  minify: MINIFY_JS,
  sourcemap: false,
  target: "es2023",
  outdir: OUT_DIR,
  bundle: true,
  logLevel: "info",
  platform: "browser",
  format: "esm",
  external: [
    "assert",
    "node:assert",
    "async_hooks",
    "node:async_hooks",
    "buffer",
    "node:buffer",
    "child_process",
    "node:child_process",
    "console",
    "node:console",
    "crypto",
    "node:crypto",
    "dgram",
    "node:dgram",
    "dns",
    "node:dns",
    "events",
    "node:events",
    "fs",
    "node:fs",
    "module",
    "node:module",
    "net",
    "node:net",
    "os",
    "node:os",
    "path",
    "node:path",
    "perf_hooks",
    "node:perf_hooks",
    "process",
    "node:process",
    "stream",
    "node:stream",
    "stream/web",
    "node:stream/web",
    "querystring",
    "node:querystring",
    "diagnostics_channel",
    "node:diagnostics_channel",
    "string_decoder",
    "node:string_decoder",
    "timers",
    "node:timers",
    "tty",
    "node:tty",
    "url",
    "node:url",
    "util",
    "node:util",
    "v8",
    "node:v8",
    "constants",
    "node:constants",
    "vm",
    "node:vm",
    "zlib",
    "node:zlib",
    "raster_runtime:hex",
    "raster_runtime:timezone",
    "raster_runtime:util",
    "raster_runtime:qjs",
    "raster_runtime:xml",
  ],
};

const requireProcessPlugin = {
  name: "require-process",
  setup(build) {
    build.onResolve({ filter: /^process\/$/ }, () => {
      return { path: "process", external: true };
    });
  },
};

async function createOutputDirectories() {
  await fs.rm(OUT_DIR, { recursive: true, force: true });
  await fs.mkdir(OUT_DIR, { recursive: true });
}

async function buildLibrary() {
  const defaultLibEsBuildOption = {
    chunkNames: "raster_runtime-[name]-runtime-[hash]",
    ...ES_BUILD_OPTIONS,
    splitting: false,
    keepNames: true,
    nodePaths: ["."],
  };

  const entryPoints = {};
  ENTRYPOINTS.forEach((entry) => {
    entryPoints[entry] = path.join(SRC_DIR, entry);
  });
  await esbuild.build({
    ...defaultLibEsBuildOption,
    entryPoints,
    plugins: [requireProcessPlugin],
  });

  const testEntryPoints = TEST_FILES.reduce((acc, entry) => {
    const { name, dir } = path.parse(entry);
    const parentDir = path.basename(dir);
    acc[path.join("__tests__", parentDir, name)] = entry;
    return acc;
  }, {});

  await esbuild.build({
    ...defaultLibEsBuildOption,
    entryPoints: testEntryPoints,
  });
}

console.log("Building...");

await createOutputDirectories();
await buildLibrary();
