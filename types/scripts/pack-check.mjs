import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.dirname(fileURLToPath(import.meta.url));
const typesDir = path.resolve(root, "..");

const result = spawnSync(
  "npm",
  ["pack", "--dry-run", "--json"],
  { cwd: typesDir, encoding: "utf8" }
);

if (result.status !== 0) {
  console.error(result.stderr || result.stdout);
  process.exit(result.status ?? 1);
}

const stdout = result.stdout.trim();
const jsonStart = stdout.indexOf("[");
if (jsonStart < 0) {
  console.error("npm pack --json did not emit a JSON array");
  console.error(stdout);
  process.exit(1);
}

const payload = JSON.parse(stdout.slice(jsonStart));
const files = new Set(
  (Array.isArray(payload) ? payload[0]?.files : payload?.files)?.map(
    (entry) => entry.path ?? entry
  ) ?? []
);

const required = [
  "index.d.ts",
  "dns/promises.d.ts",
  "fs/promises.d.ts",
  "stream/web.d.ts",
];

const missing = required.filter((name) => !files.has(name));
if (missing.length > 0) {
  console.error("npm pack is missing nested declaration files:");
  for (const name of missing) {
    console.error(`  - ${name}`);
  }
  process.exit(1);
}

console.log("npm pack includes nested declaration files");
