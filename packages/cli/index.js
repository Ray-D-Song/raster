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
  raster dev android|ios
  raster build ios

Commands:
  create <project-name>       Create a new Raster app.
  add <platforms>             Add platform shell apps. Example: android,ios,win
  dev android|ios             Start mobile bundle watch and dev server.
  build ios                   Build the iOS shell app.

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
const IOS_DEV_PORT = 14201;
const ANDROID_DEV_SCRIPT = "raster dev android";
const IOS_DEV_SCRIPT = "raster dev ios";
const ANDROID_TODO_SCRIPT =
  "node -e \"throw new Error('Raster Android build/run is not implemented in the CLI yet')\"";
const IOS_BUILD_SCRIPT = "raster build ios";

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

  if (command === "build") {
    await build(args);
    return;
  }

  throw new CliError(`Unknown command: ${command}\n\n${HELP}`);
}

async function dev(args) {
  const platform = args[0];
  if (!platform) {
    throw new CliError("Missing platform. Usage: raster dev android|ios");
  }
  if (args.length > 1) {
    throw new CliError(`Unexpected arguments for dev: ${args.slice(1).join(" ")}`);
  }
  if (platform === "android") {
    await devAndroid();
    return;
  }
  if (platform === "ios") {
    await devIos();
    return;
  }
  throw new CliError(`Unsupported dev platform: ${platform}`);
}

async function build(args) {
  const platform = args[0];
  if (!platform) {
    throw new CliError("Missing platform. Usage: raster build ios");
  }
  if (args.length > 1) {
    throw new CliError(`Unexpected arguments for build: ${args.slice(1).join(" ")}`);
  }
  if (platform !== "ios") {
    throw new CliError(`Unsupported build platform: ${platform}`);
  }

  await buildIos();
}

async function devAndroid() {
  const projectRoot = process.cwd();
  const packageJsonPath = path.join(projectRoot, "package.json");
  const androidDir = path.join(projectRoot, "android");
  await assertReadableFile(packageJsonPath, "raster dev android must be run from a project root with package.json");
  await assertReadableDir(androidDir, "raster dev android requires an android/ platform directory");
  await runMobileDevServer({
    projectRoot,
    platformName: "Android",
    platformDir: androidDir,
    port: ANDROID_DEV_PORT,
    writeDevConfig: async () => {},
    extraLogLines: [`Emulator fallback URL: http://10.0.2.2:${ANDROID_DEV_PORT}/app.js`],
    beforeWatch: () => tryAdbReverse(ANDROID_DEV_PORT),
  });
}

async function devIos() {
  const projectRoot = process.cwd();
  const packageJsonPath = path.join(projectRoot, "package.json");
  const iosDir = path.join(projectRoot, "ios");
  await assertReadableFile(packageJsonPath, "raster dev ios must be run from a project root with package.json");
  await assertReadableDir(iosDir, "raster dev ios requires an ios/ platform directory");
  await runMobileDevServer({
    projectRoot,
    platformName: "iOS",
    platformDir: iosDir,
    port: IOS_DEV_PORT,
    writeDevConfig: () => writeIosDevConfig(projectRoot),
    extraLogLines: ["Run the iOS debug app yourself from Xcode."],
  });
}

async function buildIos() {
  const projectRoot = process.cwd();
  const packageJsonPath = path.join(projectRoot, "package.json");
  const iosDir = path.join(projectRoot, "ios");
  await assertReadableFile(packageJsonPath, "raster build ios must be run from a project root with package.json");
  await assertReadableDir(iosDir, "raster build ios requires an ios/ platform directory");

  const viteBin = await resolveLocalBin(projectRoot, "vite");
  if (!viteBin) {
    throw new CliError("Failed to find local Vite binary. Install project dependencies first.");
  }
  await runCommand(viteBin, ["build"], {
    cwd: projectRoot,
    env: {
      ...process.env,
      RASTER_UNPLUGIN_SKIP_BINARY: "1",
    },
  });

  await copyIosBundleResources(projectRoot);
  await runCommand("xcodebuild", [
    "-project",
    path.join(iosDir, "RasterIOS.xcodeproj"),
    "-scheme",
    "RasterIOS",
    "-destination",
    "generic/platform=iOS Simulator",
    "build",
  ], { cwd: projectRoot });
}

async function runMobileDevServer({
  projectRoot,
  platformName,
  port,
  writeDevConfig,
  extraLogLines = [],
  beforeWatch,
}) {
  const viteBin = await resolveLocalBin(projectRoot, "vite");
  if (!viteBin) {
    throw new CliError("Failed to find local Vite binary. Install project dependencies first.");
  }

  await writeDevConfig();
  const bundlePath = path.join(projectRoot, "target", "raster", "app.js");
  const sourceMapPath = `${bundlePath}.map`;
  const devServer = createMobileDevServer({ bundlePath, sourceMapPath });
  await listen(devServer.server, port);
  devServer.start();

  console.log(`Raster ${platformName} dev server listening on http://127.0.0.1:${port}`);
  console.log(`Bundle endpoint: http://127.0.0.1:${port}/app.js`);
  for (const line of extraLogLines) {
    console.log(line);
  }
  beforeWatch?.();

  const vite = spawn(viteBin, ["build", "--watch"], {
    cwd: projectRoot,
    env: {
      ...process.env,
      RASTER_UNPLUGIN_SKIP_BINARY: "1",
    },
    stdio: "inherit",
  });

  const shutdown = () => {
    devServer.close();
    if (!vite.killed) {
      vite.kill("SIGTERM");
    }
  };
  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);

  await new Promise((resolve, reject) => {
    vite.on("error", reject);
    vite.on("exit", (code, signal) => {
      devServer.close();
      if (code === 0 || signal === "SIGTERM" || signal === "SIGINT") {
        resolve();
      } else {
        reject(new CliError(`Vite watch exited with code ${code ?? signal}`));
      }
    });
  });
}

async function writeIosDevConfig(projectRoot) {
  const rasterDir = path.join(projectRoot, "ios", "RasterIOS", "Resources", "raster");
  await fs.mkdir(rasterDir, { recursive: true });
  await fs.writeFile(
    path.join(rasterDir, "dev.json"),
    `${JSON.stringify({ urls: [`http://127.0.0.1:${IOS_DEV_PORT}/app.js`] }, null, 2)}\n`,
  );
}

async function copyIosBundleResources(projectRoot) {
  const sourceDir = path.join(projectRoot, "target", "raster");
  const targetDir = path.join(projectRoot, "ios", "RasterIOS", "Resources", "raster");
  await fs.mkdir(targetDir, { recursive: true });
  await fs.copyFile(path.join(sourceDir, "app.js"), path.join(targetDir, "app.js"));
  try {
    await fs.copyFile(path.join(sourceDir, "app.js.map"), path.join(targetDir, "app.js.map"));
  } catch (error) {
    if (error?.code !== "ENOENT") {
      throw error;
    }
  }
}

function runCommand(command, args, options) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd,
      env: options.env ?? process.env,
      stdio: "inherit",
      shell: process.platform === "win32",
    });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new CliError(`${command} ${args.join(" ")} failed with ${signal ?? code}`));
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
      packageJson.scripts["dev:ios"] = IOS_DEV_SCRIPT;
      packageJson.scripts.ios = IOS_BUILD_SCRIPT;
      packageJson.scripts["build:ios"] = IOS_BUILD_SCRIPT;
    }
  }
}

function platformScriptNames(platform) {
  if (platform === "android") {
    return ["dev:android", "android", "build:android"];
  }
  return [`dev:${platform}`, platform, `build:${platform}`];
}

function createMobileDevServer({ bundlePath, sourceMapPath }) {
  const clients = new Set();
  let currentBundleEvent = null;
  let currentBundleSignature = null;
  let bundleWatchTimer = null;

  const sendBundleEvent = (response, event) => {
    response.write(`event: bundle\n`);
    response.write(`data: ${JSON.stringify(event)}\n\n`);
  };

  const broadcastBundleEvent = (event) => {
    for (const response of clients) {
      sendBundleEvent(response, event);
    }
  };

  const refreshBundleVersion = async () => {
    try {
      const stat = await fs.stat(bundlePath);
      if (!stat.isFile()) {
        return;
      }
      const signature = `${stat.mtimeMs}:${stat.size}`;
      if (signature === currentBundleSignature) {
        return;
      }
      const body = await fs.readFile(bundlePath);
      currentBundleSignature = signature;
      currentBundleEvent = {
        version: crypto.createHash("sha256").update(body).digest("hex"),
        url: "/app.js",
      };
      broadcastBundleEvent(currentBundleEvent);
    } catch (error) {
      if (error?.code !== "ENOENT") {
        console.warn(`Failed to read Raster bundle version: ${error.message}`);
      }
    }
  };

  const server = http.createServer(async (request, response) => {
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
      if (url.pathname === "/events") {
        response.useChunkedEncodingByDefault = false;
        response.writeHead(200, {
          "cache-control": "no-cache, no-store",
          "connection": "keep-alive",
          "content-type": "text/event-stream; charset=utf-8",
          "x-accel-buffering": "no",
        });
        response.write("retry: 1000\n\n");
        clients.add(response);
        if (currentBundleEvent) {
          sendBundleEvent(response, currentBundleEvent);
        }
        request.on("close", () => {
          clients.delete(response);
        });
        return;
      }

      response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
      response.end("Not found");
    } catch (error) {
      response.writeHead(500, { "content-type": "text/plain; charset=utf-8" });
      response.end(error instanceof Error ? error.message : String(error));
    }
  });

  return {
    server,
    start() {
      void refreshBundleVersion();
      bundleWatchTimer = setInterval(() => {
        void refreshBundleVersion();
      }, 250);
    },
    close() {
      if (bundleWatchTimer) {
        clearInterval(bundleWatchTimer);
        bundleWatchTimer = null;
      }
      for (const response of clients) {
        response.end();
      }
      clients.clear();
      server.close();
    },
  };
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
