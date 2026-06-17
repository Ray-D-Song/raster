#!/usr/bin/env node

import { access, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const rootDir = path.resolve(path.dirname(__filename), "..");
const packageSwiftPath = path.join(rootDir, "Package.swift");
const configPath = path.join(rootDir, "config.json");
const distDir = path.join(rootDir, "packages/raster-ios/dist");
const remotePackageBackupPath = path.join(distDir, "Package.swift.remote");
const localXcframeworkPath = path.join(distDir, "RasterRuntime.xcframework");
const showcaseProjectPath = path.join(rootDir, "apps/showcase/ios/RasterIOS.xcodeproj/project.pbxproj");
const showcasePackageResolvedPath = path.join(
  rootDir,
  "apps/showcase/ios/RasterIOS.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved",
);
const showcaseLocalPackageRelativePath = "../../..";
const packageReferenceId = "C9CC21BD2FDBEAC00025D2F2";
const packageProductId = "C9CC21BC2FDBEAC00025D2F2";
const rasterRepositoryUrl = "https://github.com/Ray-D-Song/raster";

async function main(argv) {
  const command = argv[0];
  const skipBuild = argv.includes("--skip-build");

  if (command === "local") {
    await switchToLocal({ skipBuild });
    return;
  }
  if (command === "remote") {
    await switchToRemote();
    return;
  }
  if (command === "status") {
    await printStatus();
    return;
  }

  console.error([
    "Usage:",
    "  node scripts/ios-local-package.mjs local [--skip-build]",
    "  node scripts/ios-local-package.mjs remote",
    "  node scripts/ios-local-package.mjs status",
  ].join("\n"));
  process.exitCode = 1;
}

async function switchToLocal({ skipBuild }) {
  await saveRemotePackageSwift();
  if (!skipBuild) {
    await run("node", ["scripts/build.mjs", "--platform", "ios-xcframework"]);
  } else {
    await access(localXcframeworkPath).catch(() => {
      throw new Error(`Missing local XCFramework: ${localXcframeworkPath}`);
    });
  }

  await writePackageSwiftLocal();
  await writeShowcaseProjectLocal();
  await rm(showcasePackageResolvedPath, { force: true });
  await cleanXcodeState();

  console.log("showcase iOS now uses the local Raster Swift package.");
  console.log(`RasterRuntime binary target: ${path.relative(rootDir, localXcframeworkPath)}`);
  console.log("Open apps/showcase/ios/RasterIOS.xcodeproj or build it from the command line.");
}

async function switchToRemote() {
  await restoreRemotePackageSwift();
  await writeShowcaseProjectRemote();
  await restoreShowcasePackageResolved();
  await cleanXcodeState();

  console.log("showcase iOS now uses the remote Raster Swift package.");
}

async function printStatus() {
  const packageSwift = await readFile(packageSwiftPath, "utf8");
  const project = await readFile(showcaseProjectPath, "utf8");
  const packageMode = packageSwift.includes("path: \"packages/raster-ios/dist/RasterRuntime.xcframework\"")
    ? "local"
    : "remote";
  const showcaseMode = project.includes("XCLocalSwiftPackageReference")
    ? "local"
    : "remote";
  console.log(`Package.swift: ${packageMode}`);
  console.log(`showcase iOS project: ${showcaseMode}`);
}

async function saveRemotePackageSwift() {
  const source = await readFile(packageSwiftPath, "utf8");
  if (!source.includes("url: \"https://github.com/Ray-D-Song/raster/releases/download/")) {
    return;
  }
  await mkdir(distDir, { recursive: true });
  await writeFile(remotePackageBackupPath, source);
}

async function restoreRemotePackageSwift() {
  let source;
  try {
    source = await readFile(remotePackageBackupPath, "utf8");
  } catch {
    source = await capture("git", ["show", "HEAD:Package.swift"]);
  }
  await writeFile(packageSwiftPath, source);
  await rm(remotePackageBackupPath, { force: true });
}

async function writePackageSwiftLocal() {
  const source = await readFile(packageSwiftPath, "utf8");
  const next = replaceRasterRuntimeTarget(source, [
    `.binaryTarget(`,
    `            name: "RasterRuntime",`,
    `            path: "packages/raster-ios/dist/RasterRuntime.xcframework"`,
    `        )`,
  ].join("\n"));
  await writeFile(packageSwiftPath, next);
}

async function writeShowcaseProjectLocal() {
  const source = await readFile(showcaseProjectPath, "utf8");
  let next = replaceRemoteProjectReference(source, [
    `/* Begin XCLocalSwiftPackageReference section */`,
    `\t\t${packageReferenceId} /* XCLocalSwiftPackageReference "${showcaseLocalPackageRelativePath}" */ = {`,
    `\t\t\tisa = XCLocalSwiftPackageReference;`,
    `\t\t\trelativePath = ${showcaseLocalPackageRelativePath};`,
    `\t\t};`,
    `/* End XCLocalSwiftPackageReference section */`,
  ].join("\n"));
  next = next.replaceAll(
    `${packageReferenceId} /* XCRemoteSwiftPackageReference "raster" */`,
    `${packageReferenceId} /* XCLocalSwiftPackageReference "${showcaseLocalPackageRelativePath}" */`,
  );
  next = next.replaceAll(
    `package = ${packageReferenceId} /* XCRemoteSwiftPackageReference "raster" */;`,
    `package = ${packageReferenceId} /* XCLocalSwiftPackageReference "${showcaseLocalPackageRelativePath}" */;`,
  );
  await writeFile(showcaseProjectPath, next);
}

async function writeShowcaseProjectRemote() {
  const version = await readReleaseVersion();
  const source = await readFile(showcaseProjectPath, "utf8");
  let next = replaceLocalProjectReference(source, [
    `/* Begin XCRemoteSwiftPackageReference section */`,
    `\t\t${packageReferenceId} /* XCRemoteSwiftPackageReference "raster" */ = {`,
    `\t\t\tisa = XCRemoteSwiftPackageReference;`,
    `\t\t\trepositoryURL = "${rasterRepositoryUrl}";`,
    `\t\t\trequirement = {`,
    `\t\t\t\tkind = exactVersion;`,
    `\t\t\t\tversion = "${version}";`,
    `\t\t\t};`,
    `\t\t};`,
    `/* End XCRemoteSwiftPackageReference section */`,
  ].join("\n"));
  next = next.replaceAll(
    `${packageReferenceId} /* XCLocalSwiftPackageReference "${showcaseLocalPackageRelativePath}" */`,
    `${packageReferenceId} /* XCRemoteSwiftPackageReference "raster" */`,
  );
  next = next.replaceAll(
    `package = ${packageReferenceId} /* XCLocalSwiftPackageReference "${showcaseLocalPackageRelativePath}" */;`,
    `package = ${packageReferenceId} /* XCRemoteSwiftPackageReference "raster" */;`,
  );
  await writeFile(showcaseProjectPath, next);
}

async function restoreShowcasePackageResolved() {
  try {
    const source = await capture("git", [
      "show",
      `HEAD:${path.relative(rootDir, showcasePackageResolvedPath)}`,
    ]);
    await mkdir(path.dirname(showcasePackageResolvedPath), { recursive: true });
    await writeFile(showcasePackageResolvedPath, source);
  } catch {
    await rm(showcasePackageResolvedPath, { force: true });
  }
}

function replaceRasterRuntimeTarget(source, replacement) {
  const next = source.replace(
    /\.binaryTarget\(\s*name: "RasterRuntime",[\s\S]*?\n        \)/,
    replacement,
  );
  if (next === source) {
    throw new Error("Package.swift does not contain the expected RasterRuntime binary target");
  }
  return next;
}

function replaceRemoteProjectReference(source, replacement) {
  if (source.includes("XCLocalSwiftPackageReference")) {
    return source;
  }
  const next = source.replace(
    /\/\* Begin XCRemoteSwiftPackageReference section \*\/[\s\S]*?\/\* End XCRemoteSwiftPackageReference section \*\//,
    replacement,
  );
  if (next === source) {
    throw new Error("showcase Xcode project does not contain a remote Swift package reference");
  }
  return next;
}

function replaceLocalProjectReference(source, replacement) {
  if (source.includes("XCRemoteSwiftPackageReference")) {
    return source;
  }
  const next = source.replace(
    /\/\* Begin XCLocalSwiftPackageReference section \*\/[\s\S]*?\/\* End XCLocalSwiftPackageReference section \*\//,
    replacement,
  );
  if (next === source) {
    throw new Error("showcase Xcode project does not contain a local Swift package reference");
  }
  return next;
}

async function readReleaseVersion() {
  const config = JSON.parse(await readFile(configPath, "utf8"));
  if (typeof config.version !== "string" || config.version.trim() === "") {
    throw new Error("config.json must contain a non-empty version string");
  }
  return config.version;
}

async function cleanXcodeState() {
  await rm(path.join(
    process.env.HOME,
    "Library/Developer/Xcode/DerivedData/RasterIOS-dykmiexvxbtzxzapekfsduxagtzx",
  ), { recursive: true, force: true });
}

function run(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: rootDir,
      stdio: "inherit",
      ...options,
    });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${command} ${args.join(" ")} failed with ${signal ?? code}`));
      }
    });
  });
}

function capture(command, args) {
  return new Promise((resolve, reject) => {
    let stdout = "";
    let stderr = "";
    const child = spawn(command, args, {
      cwd: rootDir,
      stdio: ["ignore", "pipe", "pipe"],
    });
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0) {
        resolve(stdout);
      } else {
        reject(new Error(`${command} ${args.join(" ")} failed with ${signal ?? code}: ${stderr}`));
      }
    });
  });
}

main(process.argv.slice(2)).catch((error) => {
  console.error(error.message);
  process.exitCode = 1;
});
