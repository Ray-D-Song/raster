import { readFile } from "node:fs/promises";
import { build } from "esbuild";

const rootPackage = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
const version = rootPackage.version;

if (typeof version !== "string" || version.length === 0) {
  throw new Error("root package.json must define a non-empty version");
}

await build({
  entryPoints: ["src/runtime-bundle.ts"],
  bundle: true,
  format: "iife",
  globalName: "__RasterBundle",
  define: {
    "process.env.NODE_ENV": JSON.stringify("production"),
  },
  banner: {
    js: `globalThis.__rasterRendererVersion = globalThis.__rasterRendererVersion ?? ${JSON.stringify(version)};`,
  },
  footer: {
    js: "globalThis.__RasterBundle = __RasterBundle;",
  },
  outfile: "../../src/runtime/js/generated/runtime_bundle.js",
});
