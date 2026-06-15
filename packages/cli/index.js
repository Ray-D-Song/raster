#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import crypto from "node:crypto";
import { constants as fsConstants } from "node:fs";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const defaultTemplateDir = path.join(__dirname, "templates", "default");
const platformTemplatesDir = path.join(__dirname, "templates", "platforms");

const HELP = `Raster CLI

Usage:
  raster create <project-name>
  raster add <platforms>
  raster dev android

Commands:
  create <project-name>       Create a new Raster app.
  add <platforms>             Add platform shell apps. Example: android,ios,win
  dev android                 Start Android bundle watch and dev server.

Supported platform inputs:
  android, ios, win, windows, osx, macos, linux
`;

const PLATFORM_ALIASES = new Map([
  ["android", "android"],
  ["ios", "ios"],
  ["win", "windows"],
  ["windows", "windows"],
  ["osx", "macos"],
  ["macos", "macos"],
  ["linux", "linux"],
]);

const IMPLEMENTED_PLATFORMS = new Set(["android", "ios"]);
const ANDROID_DEV_PORT = 14200;
const ANDROID_DEV_SCRIPT = "raster dev android";
const ANDROID_TODO_SCRIPT =
  "node -e \"throw new Error('Raster Android build/run is not implemented in the CLI yet')\"";
const IOS_TODO_SCRIPT =
  "node -e \"throw new Error('Raster iOS build/run is not implemented in the CLI yet')\"";

class CliError extends Error {
  constructor(message) {
    super(message);
    this.name = "CliError";
  }
}

async function main(argv) {
  const [command, ...args] = argv;

  if (!command || command === "--help" || command === "-h") {
    console.log(HELP);
    return;
  }

  if (command === "create") {
    await createProject(args);
    return;
  }

  if (command === "add") {
    await addPlatforms(args);
    return;
  }

  if (command === "dev") {
    await dev(args);
    return;
  }

  throw new CliError(`Unknown command: ${command}\n\n${HELP}`);
}

async function dev(args) {
  const platform = args[0];
  if (!platform) {
    throw new CliError("Missing platform. Usage: raster dev android");
  }
  if (args.length > 1) {
    throw new CliError(`Unexpected arguments for dev: ${args.slice(1).join(" ")}`);
  }
  if (platform !== "android") {
    throw new CliError(`Unsupported dev platform: ${platform}`);
  }

  await devAndroid();
}

async function devAndroid() {
  const projectRoot = process.cwd();
  const packageJsonPath = path.join(projectRoot, "package.json");
  const androidDir = path.join(projectRoot, "android");
  await assertReadableFile(packageJsonPath, "raster dev android must be run from a project root with package.json");
  await assertReadableDir(androidDir, "raster dev android requires an android/ platform directory");

  const viteBin = await resolveLocalBin(projectRoot, "vite");
  if (!viteBin) {
    throw new CliError("Failed to find local Vite binary. Install project dependencies first.");
  }

  const bundlePath = path.join(projectRoot, "dist", "raster", "app.js");
  const sourceMapPath = `${bundlePath}.map`;
  const server = createAndroidDevServer({ bundlePath, sourceMapPath });
  await listen(server, ANDROID_DEV_PORT);

  console.log(`Raster Android dev server listening on http://127.0.0.1:${ANDROID_DEV_PORT}`);
  console.log(`Bundle endpoint: http://127.0.0.1:${ANDROID_DEV_PORT}/app.js`);
  console.log("Run the Android debug app yourself from Android Studio or Gradle.");
  console.log(`Emulator fallback URL: http://10.0.2.2:${ANDROID_DEV_PORT}/app.js`);

  tryAdbReverse(ANDROID_DEV_PORT);

  const vite = spawn(viteBin, ["build", "--watch"], {
    cwd: projectRoot,
    env: {
      ...process.env,
      RASTER_UNPLUGIN_SKIP_BINARY: "1",
    },
    stdio: "inherit",
  });

  const shutdown = () => {
    server.close();
    if (!vite.killed) {
      vite.kill("SIGTERM");
    }
  };
  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);

  await new Promise((resolve, reject) => {
    vite.on("error", reject);
    vite.on("exit", (code, signal) => {
      server.close();
      if (code === 0 || signal === "SIGTERM" || signal === "SIGINT") {
        resolve();
      } else {
        reject(new CliError(`Vite watch exited with code ${code ?? signal}`));
      }
    });
  });
}

async function createProject(args) {
  const projectName = args[0];
  if (!projectName) {
    throw new CliError("Missing project name. Usage: raster create <project-name>");
  }
  if (args.length > 1) {
    throw new CliError(`Unexpected arguments for create: ${args.slice(1).join(" ")}`);
  }

  const targetDir = path.resolve(process.cwd(), projectName);
  await assertPathMissing(targetDir, `Target directory already exists: ${targetDir}`);

  const packageName = toValidPackageName(projectName);
  if (!packageName) {
    throw new CliError(`Project name cannot be converted to a valid package name: ${projectName}`);
  }

  await copyTemplate(defaultTemplateDir, targetDir, {
    __RASTER_APP_NAME__: packageName,
  });

  console.log(`Created Raster app in ${path.relative(process.cwd(), targetDir) || "."}`);
  console.log("Next steps:");
  console.log(`  cd ${projectName}`);
  console.log("  install dependencies");
  console.log("  npm run build");
}

async function addPlatforms(args) {
  const platformArg = args[0];
  if (!platformArg) {
    throw new CliError("Missing platforms. Usage: raster add android,ios,win,osx,linux");
  }
  if (args.length > 1) {
    throw new CliError(`Unexpected arguments for add: ${args.slice(1).join(" ")}`);
  }

  const projectRoot = process.cwd();
  const packageJsonPath = path.join(projectRoot, "package.json");
  await assertReadableFile(packageJsonPath, "raster add must be run from a project root with package.json");

  const platforms = parsePlatforms(platformArg);
  const notImplemented = platforms.filter((platform) => !IMPLEMENTED_PLATFORMS.has(platform));
  if (notImplemented.length > 0) {
    throw new CliError(
      notImplemented
        .map((platform) => `${platform} platform template is not implemented yet`)
        .join("\n"),
    );
  }

  const plannedCopies = platforms.map((platform) => ({
    platform,
    source: path.join(platformTemplatesDir, platform),
    target: path.join(projectRoot, platform),
  }));

  for (const copy of plannedCopies) {
    await assertReadableDir(copy.source, `${copy.platform} platform template is missing: ${copy.source}`);
    await assertPathMissing(copy.target, `Platform directory already exists: ${copy.target}`);
  }

  const packageJson = await readPackageJson(packageJsonPath);
  validateScriptSlots(packageJson, platforms);

  for (const copy of plannedCopies) {
    await copyTemplate(copy.source, copy.target, {});
  }

  updatePlatformScripts(packageJson, platforms);
  await fs.writeFile(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);

  console.log(`Added platforms: ${platforms.join(", ")}`);
}

function parsePlatforms(raw) {
  const parts = raw.split(",").map((part) => part.trim());
  if (parts.some((part) => part.length === 0)) {
    throw new CliError(`Invalid platform list: ${raw}`);
  }

  const seen = new Set();
  const platforms = [];
  for (const input of parts) {
    const normalized = PLATFORM_ALIASES.get(input.toLowerCase());
    if (!normalized) {
      throw new CliError(`Unknown platform: ${input}`);
    }
    if (seen.has(normalized)) {
      throw new CliError(`Duplicate platform: ${normalized}`);
    }
    seen.add(normalized);
    platforms.push(normalized);
  }

  return platforms;
}

async function copyTemplate(sourceDir, targetDir, replacements) {
  if (Object.keys(replacements).length === 0) {
    await fs.cp(sourceDir, targetDir, {
      recursive: true,
      force: false,
      errorOnExist: true,
    });
    return;
  }

  await fs.mkdir(targetDir, { recursive: true });
  await copyDirectoryEntries(sourceDir, targetDir, replacements);
}

async function copyDirectoryEntries(sourceDir, targetDir, replacements) {
  const entries = await fs.readdir(sourceDir, { withFileTypes: true });
  for (const entry of entries) {
    const sourcePath = path.join(sourceDir, entry.name);
    const targetPath = path.join(targetDir, entry.name);
    if (entry.isDirectory()) {
      await fs.mkdir(targetPath, { recursive: true });
      await copyDirectoryEntries(sourcePath, targetPath, replacements);
    } else if (entry.isFile()) {
      await copyFileWithReplacements(sourcePath, targetPath, replacements);
    } else if (entry.isSymbolicLink()) {
      const linkTarget = await fs.readlink(sourcePath);
      await fs.symlink(linkTarget, targetPath);
    }
  }
}

async function copyFileWithReplacements(sourcePath, targetPath, replacements) {
  const buffer = await fs.readFile(sourcePath);
  let content = buffer.toString("utf8");
  for (const [token, value] of Object.entries(replacements)) {
    content = content.split(token).join(value);
  }
  await fs.writeFile(targetPath, content);
}

async function readPackageJson(packageJsonPath) {
  try {
    return JSON.parse(await fs.readFile(packageJsonPath, "utf8"));
  } catch (error) {
    throw new CliError(`Failed to read package.json: ${error.message}`);
  }
}

function validateScriptSlots(packageJson, platforms) {
  const scripts = packageJson.scripts ?? {};
  for (const platform of platforms) {
    const names = platformScriptNames(platform);
    for (const name of names) {
      if (Object.prototype.hasOwnProperty.call(scripts, name)) {
        throw new CliError(`package.json already has a "${name}" script`);
      }
    }
  }
}

function updatePlatformScripts(packageJson, platforms) {
  packageJson.scripts = packageJson.scripts ?? {};
  for (const platform of platforms) {
    if (platform === "android") {
      packageJson.scripts["dev:android"] = ANDROID_DEV_SCRIPT;
      packageJson.scripts.android = ANDROID_TODO_SCRIPT;
      packageJson.scripts["build:android"] = ANDROID_TODO_SCRIPT;
    } else if (platform === "ios") {
      packageJson.scripts.ios = IOS_TODO_SCRIPT;
      packageJson.scripts["build:ios"] = IOS_TODO_SCRIPT;
    }
  }
}

function platformScriptNames(platform) {
  if (platform === "android") {
    return ["dev:android", "android", "build:android"];
  }
  return [platform, `build:${platform}`];
}

function createAndroidDevServer({ bundlePath, sourceMapPath }) {
  return http.createServer(async (request, response) => {
    try {
      const url = new URL(request.url ?? "/", "http://127.0.0.1");
      if (url.pathname === "/app.js") {
        await sendFile(response, bundlePath, "application/javascript; charset=utf-8");
        return;
      }
      if (url.pathname === "/app.js.map") {
        await sendFile(response, sourceMapPath, "application/json; charset=utf-8");
        return;
      }
      if (url.pathname === "/version") {
        await sendBundleVersion(response, bundlePath);
        return;
      }

      response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
      response.end("Not found");
    } catch (error) {
      response.writeHead(500, { "content-type": "text/plain; charset=utf-8" });
      response.end(error instanceof Error ? error.message : String(error));
    }
  });
}

async function sendFile(response, filePath, contentType) {
  try {
    const body = await fs.readFile(filePath);
    response.writeHead(200, {
      "cache-control": "no-store",
      "connection": "close",
      "content-length": String(body.byteLength),
      "content-type": contentType,
    });
    response.end(body);
  } catch (error) {
    if (error?.code === "ENOENT") {
      response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
      response.end(`Bundle has not been built yet: ${filePath}`);
      return;
    }
    throw error;
  }
}

async function sendBundleVersion(response, bundlePath) {
  try {
    const body = await fs.readFile(bundlePath);
    const hash = crypto.createHash("sha256").update(body).digest("hex");
    response.writeHead(200, {
      "cache-control": "no-store",
      "connection": "close",
      "content-length": String(Buffer.byteLength(hash)),
      "content-type": "text/plain; charset=utf-8",
    });
    response.end(hash);
  } catch (error) {
    if (error?.code === "ENOENT") {
      response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
      response.end("missing");
      return;
    }
    throw error;
  }
}

async function listen(server, port) {
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, "127.0.0.1", () => {
      server.off("error", reject);
      resolve();
    });
  });
}

async function resolveLocalBin(projectRoot, name) {
  const extension = process.platform === "win32" ? ".cmd" : "";
  const binPath = path.join(projectRoot, "node_modules", ".bin", `${name}${extension}`);
  try {
    await fs.access(binPath, fsConstants.X_OK);
    return binPath;
  } catch {
    return null;
  }
}

function tryAdbReverse(port) {
  const devices = spawnSync("adb", ["devices"], {
    encoding: "utf8",
    timeout: 2000,
  });
  if (devices.error || devices.status !== 0) {
    console.log(`For a real device, run: adb reverse tcp:${port} tcp:${port}`);
    return;
  }

  const hasDevice = devices.stdout
    .split(/\r?\n/)
    .some((line) => /\tdevice$/.test(line.trim()));
  if (!hasDevice) {
    console.log(`For a real device, run: adb reverse tcp:${port} tcp:${port}`);
    return;
  }

  const reverse = spawnSync("adb", ["reverse", `tcp:${port}`, `tcp:${port}`], {
    encoding: "utf8",
    timeout: 2000,
  });
  if (reverse.status === 0) {
    console.log(`Configured adb reverse tcp:${port} tcp:${port}`);
  } else {
    console.log(`adb reverse failed. Run manually: adb reverse tcp:${port} tcp:${port}`);
  }
}

function toValidPackageName(projectName) {
  return projectName
    .trim()
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/^[._]/, "")
    .replace(/[^a-z0-9-~]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}

async function assertPathMissing(targetPath, message) {
  try {
    await fs.access(targetPath, fsConstants.F_OK);
  } catch {
    return;
  }
  throw new CliError(message);
}

async function assertReadableFile(filePath, message) {
  try {
    const stat = await fs.stat(filePath);
    if (stat.isFile()) {
      return;
    }
  } catch {
    // handled below
  }
  throw new CliError(message);
}

async function assertReadableDir(dirPath, message) {
  try {
    const stat = await fs.stat(dirPath);
    if (stat.isDirectory()) {
      return;
    }
  } catch {
    // handled below
  }
  throw new CliError(message);
}

main(process.argv.slice(2)).catch((error) => {
  if (error instanceof CliError) {
    console.error(error.message);
    process.exitCode = 1;
    return;
  }
  console.error(error);
  process.exitCode = 1;
});
