import { spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";

const [name, rasterPath] = process.argv.slice(2);
const root = process.cwd();
const cases = {
  next: {
    directory: "compat/next",
    command: "node_modules/next/dist/bin/next",
    args: ["build"],
    output: ".next",
    checks: [
      ["BUILD_ID"],
      ["routes-manifest.json"],
      ["server", "app", "page.js"],
      ["server", "app-paths-manifest.json"],
      ["static"]
    ]
  },
  "vite-plus": {
    directory: "compat/vite-plus",
    command: "node_modules/vite-plus/bin/vp",
    args: ["build"],
    output: "dist",
    checks: [
      ["index.js"],
      ["index.cjs"],
      ["style.css"],
      [".vite", "manifest.json"]
    ]
  }
};

const testCase = cases[name];
if (!testCase || !rasterPath) {
  throw new Error("Usage: node compat/run.mjs <next|vite-plus> <raster-runtime>");
}

const directory = path.join(root, testCase.directory);
const output = path.join(directory, testCase.output);
const raster = path.resolve(root, rasterPath);
const command = path.join(directory, testCase.command);
const logPath = path.join(directory, "compat.log");

await fs.rm(output, { recursive: true, force: true });

const result = await new Promise((resolve, reject) => {
  const child = spawn(raster, [command, ...testCase.args], {
    cwd: directory,
    env: { ...process.env, NEXT_TELEMETRY_DISABLED: "1" },
    stdio: ["ignore", "pipe", "pipe"]
  });
  let stdout = "";
  let stderr = "";
  child.stdout.on("data", (chunk) => (stdout += chunk));
  child.stderr.on("data", (chunk) => (stderr += chunk));
  child.on("error", reject);
  child.on("close", (code, signal) => resolve({ code, signal, stdout, stderr }));
});

const log = `$ ${raster} ${command} ${testCase.args.join(" ")}\n\nstdout:\n${result.stdout}\n\nstderr:\n${result.stderr}\n`;
await fs.writeFile(logPath, log);
process.stdout.write(result.stdout);
process.stderr.write(result.stderr);

const outputExists = await fs
  .access(output)
  .then(() => true)
  .catch(() => false);

if (result.code !== 0) {
  throw new Error(`${name} build exited with ${result.code ?? result.signal}`);
}

if (!outputExists) {
  throw new Error(
    `${name} exited 0 but produced no ${testCase.output}/ directory. ` +
      `stdout empty=${result.stdout.length === 0}, stderr empty=${result.stderr.length === 0}. ` +
      `See ${path.relative(root, logPath)} for the captured Raster child output.`
  );
}

for (const segments of testCase.checks) {
  await fs.access(path.join(output, ...segments));
}

if (name === "next") {
  const appPaths = await fs.readFile(path.join(output, "server", "app-paths-manifest.json"), "utf8");
  if (!appPaths.includes("/api/health") || !appPaths.includes("/posts/[id]")) {
    throw new Error("Next build did not emit the expected App Router routes");
  }
} else {
  const [esm, cjs, css, manifest] = await Promise.all([
    fs.readFile(path.join(output, "index.js"), "utf8"),
    fs.readFile(path.join(output, "index.cjs"), "utf8"),
    fs.readFile(path.join(output, "style.css"), "utf8"),
    fs.readFile(path.join(output, ".vite", "manifest.json"), "utf8")
  ]);
  if (!esm.includes("Button") || !cjs.includes("Button") || !css.includes(".raster-button") || !manifest.includes("src/index.tsx")) {
    throw new Error("Vite+ build output is missing an expected library artifact");
  }
}

console.log(`${name} compatibility build passed`);
