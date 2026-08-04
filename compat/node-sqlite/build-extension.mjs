import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const vendor = path.resolve(here, "../../modules/raster_runtime_sqlite/vendor");
const buildDir = path.join(here, "build");
fs.mkdirSync(buildDir, { recursive: true });

let output;
let args;
if (process.platform === "win32") {
  output = path.join(buildDir, "sqlite_extension.dll");
  args = ["-shared", `-I${vendor}`, "-o", output, path.join(here, "extension.c")];
} else if (process.platform === "darwin") {
  output = path.join(buildDir, "sqlite_extension.dylib");
  args = [
    "-dynamiclib",
    "-fPIC",
    `-I${vendor}`,
    "-o",
    output,
    path.join(here, "extension.c"),
  ];
} else {
  output = path.join(buildDir, "sqlite_extension.so");
  args = [
    "-shared",
    "-fPIC",
    `-I${vendor}`,
    "-o",
    output,
    path.join(here, "extension.c"),
  ];
}

const cc = process.env.CC || "cc";
const result = spawnSync(cc, args, { stdio: "inherit" });
if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

console.log(`built ${output}`);
