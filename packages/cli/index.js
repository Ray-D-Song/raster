#!/usr/bin/env node

import { constants as fsConstants } from "node:fs";
import fs from "node:fs/promises";
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

Commands:
  create <project-name>       Create a new Raster app.
  add <platforms>             Add platform shell apps. Example: android,ios,win

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

const IMPLEMENTED_PLATFORMS = new Set(["android"]);
const ANDROID_TODO_SCRIPT =
  "node -e \"throw new Error('Raster Android build/run is not implemented in the CLI yet')\"";

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

  throw new CliError(`Unknown command: ${command}\n\n${HELP}`);
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
      packageJson.scripts.android = ANDROID_TODO_SCRIPT;
      packageJson.scripts["build:android"] = ANDROID_TODO_SCRIPT;
    }
  }
}

function platformScriptNames(platform) {
  if (platform === "android") {
    return ["android", "build:android"];
  }
  return [platform, `build:${platform}`];
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
