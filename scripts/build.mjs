#!/usr/bin/env node

import { access, chmod, copyFile, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const rootDir = path.resolve(path.dirname(__filename), "..");
const configPath = path.join(rootDir, "config.json");

const platforms = [
  {
    id: "darwin-arm64",
    rustTarget: "aarch64-apple-darwin",
    packageDir: "packages/raster-bin-darwin-arm64",
    packageBinary: "bin/raster",
    cargoBinary: "raster",
    os: "darwin",
    cpu: "arm64",
    tools: [],
  },
  {
    id: "darwin-x64",
    rustTarget: "x86_64-apple-darwin",
    packageDir: "packages/raster-bin-darwin-x64",
    packageBinary: "bin/raster",
    cargoBinary: "raster",
    os: "darwin",
    cpu: "x64",
    tools: [],
  },
  {
    id: "linux-arm64",
    rustTarget: "aarch64-unknown-linux-gnu",
    packageDir: "packages/raster-bin-linux-arm64",
    packageBinary: "bin/raster",
    cargoBinary: "raster",
    os: "linux",
    cpu: "arm64",
    tools: [],
  },
  {
    id: "linux-x64",
    rustTarget: "x86_64-unknown-linux-gnu",
    packageDir: "packages/raster-bin-linux-x64",
    packageBinary: "bin/raster",
    cargoBinary: "raster",
    os: "linux",
    cpu: "x64",
    tools: [],
  },
  {
    id: "win32-x64",
    rustTarget: "x86_64-pc-windows-msvc",
    packageDir: "packages/raster-bin-win32-x64",
    packageBinary: "bin/raster.exe",
    cargoBinary: "raster.exe",
    os: "win32",
    cpu: "x64",
    tools: [],
  },
  {
    id: "win32-arm64",
    rustTarget: "aarch64-pc-windows-msvc",
    packageDir: "packages/raster-bin-win32-arm64",
    packageBinary: "bin/raster.exe",
    cargoBinary: "raster.exe",
    os: "win32",
    cpu: "arm64",
    tools: [],
  },
  {
    id: "android-arm64",
    rustTarget: "aarch64-linux-android",
    androidAbi: "arm64-v8a",
    androidApi: "26",
    androidJniLibsDir: "packages/raster-android/src/main/jniLibs",
    androidLibrary: "arm64-v8a/libraster.so",
    tools: [
      {
        command: "cargo-ndk",
        install: {
          default: ["cargo", ["install", "cargo-ndk"]],
        },
      },
      {
        command: "gradle",
        install: {
          darwin: brewFormula("gradle"),
        },
      },
    ],
  },
  {
    id: "ios-arm64-device",
    rustTarget: "aarch64-apple-ios",
    iosLibrary: "libraster.a",
    tools: [],
  },
  {
    id: "ios-arm64-simulator",
    rustTarget: "aarch64-apple-ios-sim",
    iosLibrary: "libraster.a",
    tools: [],
  },
  {
    id: "ios-x64-simulator",
    rustTarget: "x86_64-apple-ios",
    iosLibrary: "libraster.a",
    tools: [],
  },
  {
    id: "ios-xcframework",
    iosXcframework: true,
    rustTargets: ["aarch64-apple-ios", "aarch64-apple-ios-sim", "x86_64-apple-ios"],
    tools: [],
  },
];

function brewFormula(name) {
  return {
    trust: ["brew", ["trust", "--formula", name]],
    install: ["brew", ["install", "--no-ask", name]],
  };
}

async function main(argv) {
  const args = parseArgs(argv);
  if (args.list) {
    for (const platform of platforms) {
      console.log(`${platform.id}\t${(platform.rustTargets ?? [platform.rustTarget]).filter(Boolean).join(",")}`);
    }
    return;
  }

  const selected = args.platforms.length > 0
    ? platforms.filter((platform) => args.platforms.includes(platform.id))
    : defaultHostPlatforms();
  const missing = args.platforms.filter(
    (id) => !platforms.some((platform) => platform.id === id),
  );
  if (missing.length > 0) {
    throw new Error(`Unknown platform(s): ${missing.join(", ")}`);
  }

  for (const platform of selected) {
    validatePlatformHost(platform);
  }

  await run("pnpm", ["--dir", "packages/raster", "run", "build:runtime"]);

  for (const platform of selected) {
    await ensureRustTargets([platform]);
    await ensurePlatformTools([platform]);
    await buildPlatform(platform, args);
  }

  console.log(`Built ${selected.length} Raster platform binary package(s).`);
}

function parseArgs(argv) {
  const args = {
    platforms: [],
    installTargets: false,
    list: false,
    cargoArgs: [],
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--platform") {
      const value = argv[index + 1];
      if (!value) {
        throw new Error("--platform requires a comma-separated value");
      }
      args.platforms.push(...splitPlatforms(value));
      index += 1;
    } else if (arg.startsWith("--platform=")) {
      args.platforms.push(...splitPlatforms(arg.slice("--platform=".length)));
    } else if (arg === "--list") {
      args.list = true;
    } else if (arg === "--") {
      args.cargoArgs.push(...argv.slice(index + 1));
      break;
    } else {
      throw new Error(`Unknown build argument: ${arg}`);
    }
  }

  args.platforms = [...new Set(args.platforms)];
  return args;
}

function splitPlatforms(value) {
  const platforms = value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
  if (platforms.length === 0) {
    throw new Error("--platform requires at least one platform");
  }
  return platforms;
}

function validatePlatformHost(platform) {
  if (platform.iosLibrary || platform.iosXcframework) {
    if (process.platform !== "darwin") {
      throw new Error(`${platform.id} can only be built from macOS hosts`);
    }
    return;
  }

  if (platform.id === "android-arm64") {
    if (process.platform !== "linux" && process.platform !== "darwin") {
      throw new Error("android-arm64 can only be built from Linux or macOS hosts");
    }
    return;
  }

  if (platform.os !== process.platform || platform.cpu !== normalizedHostArch()) {
    throw new Error([
      `${platform.id} must be built on a matching native host.`,
      `Current host is ${process.platform}-${normalizedHostArch()}.`,
      "CI release builds use native matrix runners; local builds default to the current host platform.",
    ].join(" "));
  }
}

function defaultHostPlatforms() {
  const os = process.platform;
  const cpu = normalizedHostArch();
  const platform = platforms.find((item) => item.os === os && item.cpu === cpu);
  if (!platform) {
    throw new Error(`No default Raster binary platform for current host: ${os}-${cpu}`);
  }
  return [platform];
}

function normalizedHostArch() {
  if (process.arch === "x64") {
    return "x64";
  }
  if (process.arch === "arm64") {
    return "arm64";
  }
  return process.arch;
}

async function ensureRustTargets(selected) {
  const installed = new Set((await capture("rustup", ["target", "list", "--installed"]))
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean));
  const missing = selected
    .flatMap((platform) => platform.rustTargets ?? [platform.rustTarget])
    .filter(Boolean)
    .filter((target) => !installed.has(target));

  if (missing.length === 0) {
    return;
  }

  console.log(`Installing missing Rust target(s): ${missing.join(", ")}`);
  await run("rustup", ["target", "add", ...missing]);
  const afterInstall = new Set((await capture("rustup", ["target", "list", "--installed"]))
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean));
  const stillMissing = missing.filter((target) => !afterInstall.has(target));
  if (stillMissing.length > 0) {
    throw new Error(`Failed to install Rust target(s): ${stillMissing.join(", ")}`);
  }
}

async function ensurePlatformTools(selected) {
  const tools = uniqueTools(selected.flatMap((platform) => platform.tools ?? []));
  for (const tool of tools) {
    if (await commandExists(tool.command)) {
      continue;
    }

    const install = tool.install[process.platform];
    const defaultInstall = tool.install.default;
    if (!install && !defaultInstall) {
      throw new Error(
        `Missing required tool ${tool.command}, and no installer is configured for ${process.platform}`,
      );
    }

    await runInstallStep(install ?? defaultInstall, `Installing missing tool ${tool.command}`);
    if (!(await commandExists(tool.command))) {
      throw new Error(`Installed ${tool.command}, but it is still not available in PATH`);
    }
  }
}

async function runInstallStep(install, label) {
  const steps = install.trust ? [install.trust, install.install] : [install];
  for (const step of steps) {
    const [command, args] = step;
    console.log(`${label}: ${command} ${args.join(" ")}`);
    await run(command, args, { env: installEnv(command) });
  }
}

function installEnv(command) {
  if (command !== "brew") {
    return {};
  }

  return {
    HOMEBREW_NO_ASK: "1",
    HOMEBREW_NO_ENV_HINTS: "1",
  };
}

function uniqueTools(tools) {
  const seen = new Map();
  for (const tool of tools) {
    if (!seen.has(tool.command)) {
      seen.set(tool.command, tool);
    }
  }
  return [...seen.values()];
}

async function commandExists(command) {
  const lookup = process.platform === "win32" ? "where" : "sh";
  const args = process.platform === "win32" ? [command] : ["-c", `command -v ${shellQuote(command)}`];
  try {
    await capture(lookup, args);
    return true;
  } catch {
    return false;
  }
}

function shellQuote(value) {
  return `'${value.replaceAll("'", "'\\''")}'`;
}

async function buildPlatform(platform, args) {
  console.log(`Building ${platform.id} (${platform.rustTarget})`);
  if (platform.iosXcframework) {
    await buildIosXcframework(args);
    return;
  }
  if (platform.iosLibrary) {
    await buildIosLibrary(platform, args);
    return;
  }
  if (platform.androidAbi) {
    await buildAndroidPlatform(platform, args);
    return;
  }

  const env = await targetBuildEnv(platform);
  const cargoArgs = [
    platform.rustcLinkArgs?.length ? "rustc" : "build",
    "--release",
    "--bin",
    "raster",
    "--target",
    platform.rustTarget,
    ...args.cargoArgs,
  ];
  if (platform.rustcLinkArgs?.length) {
    cargoArgs.push("--", ...platform.rustcLinkArgs);
  }
  await run("cargo", cargoArgs, { env });

  const source = path.join(
    rootDir,
    "target",
    platform.rustTarget,
    "release",
    platform.cargoBinary,
  );
  const destination = path.join(rootDir, platform.packageDir, platform.packageBinary);
  await access(source).catch(() => {
    throw new Error(`Cargo build did not produce expected binary: ${source}`);
  });

  await writeBinaryPackageManifest(platform);
  await mkdir(path.dirname(destination), { recursive: true });
  await rm(destination, { force: true });
  await copyFile(source, destination);
  if (platform.packageBinary.endsWith("/raster")) {
    await chmod(destination, 0o755);
  }
  console.log(`Copied ${source} -> ${destination}`);
}

async function buildIosLibrary(platform, args) {
  const env = await targetBuildEnv(platform);
  await run("cargo", [
    "build",
    "--release",
    "--lib",
    "--target",
    platform.rustTarget,
    ...args.cargoArgs,
  ], { env });

  const source = path.join(
    rootDir,
    "target",
    platform.rustTarget,
    "release",
    platform.iosLibrary,
  );
  await access(source).catch(() => {
    throw new Error(`Cargo build did not produce expected iOS library: ${source}`);
  });
  console.log(`Built iOS library ${source}`);
}

async function buildIosXcframework(args) {
  const device = platforms.find((platform) => platform.id === "ios-arm64-device");
  const simArm64 = platforms.find((platform) => platform.id === "ios-arm64-simulator");
  const simX64 = platforms.find((platform) => platform.id === "ios-x64-simulator");
  for (const platform of [device, simArm64, simX64]) {
    await buildIosLibrary(platform, args);
  }

  const distDir = path.join(rootDir, "packages/raster-ios/dist");
  const includeDir = path.join(rootDir, "packages/raster-ios/include");
  const deviceLibrary = path.join(rootDir, "target/aarch64-apple-ios/release/libraster.a");
  const simArm64Library = path.join(rootDir, "target/aarch64-apple-ios-sim/release/libraster.a");
  const simX64Library = path.join(rootDir, "target/x86_64-apple-ios/release/libraster.a");
  const simulatorDir = path.join(rootDir, "target/ios-simulator/release");
  const simulatorLibrary = path.join(simulatorDir, "libraster.a");
  const xcframeworkPath = path.join(distDir, "RasterRuntime.xcframework");
  const zipPath = path.join(distDir, "RasterRuntime.xcframework.zip");

  await mkdir(simulatorDir, { recursive: true });
  await mkdir(distDir, { recursive: true });
  await rm(simulatorLibrary, { force: true });
  await run("xcrun", ["lipo", "-create", simArm64Library, simX64Library, "-output", simulatorLibrary]);

  await rm(xcframeworkPath, { recursive: true, force: true });
  await rm(zipPath, { force: true });
  await run("xcodebuild", [
    "-create-xcframework",
    "-library",
    deviceLibrary,
    "-headers",
    includeDir,
    "-library",
    simulatorLibrary,
    "-headers",
    includeDir,
    "-output",
    xcframeworkPath,
  ]);

  await run("ditto", [
    "-c",
    "-k",
    "--sequesterRsrc",
    "--keepParent",
    "RasterRuntime.xcframework",
    "RasterRuntime.xcframework.zip",
  ], { cwd: distDir });
  const checksum = (await capture("swift", ["package", "compute-checksum", zipPath])).trim();
  await writeFile(path.join(distDir, "RasterRuntime.xcframework.checksum"), `${checksum}\n`);
  await updatePackageSwiftChecksum(await readReleaseVersion(), checksum);
  console.log(`Built iOS XCFramework ${zipPath}`);
  console.log(`Swift package checksum ${checksum}`);
}

async function buildAndroidPlatform(platform, args) {
  const outputDir = path.join(rootDir, platform.androidJniLibsDir);
  const env = await androidBuildEnv();
  await cleanAndroidCmakeCaches(platform);
  await run("cargo", [
    "ndk",
    "-t",
    platform.androidAbi,
    "-P",
    platform.androidApi,
    "-o",
    outputDir,
    "build",
    "--release",
    "--lib",
    ...args.cargoArgs,
  ], { env });

  const destination = path.join(outputDir, platform.androidLibrary);
  await access(destination).catch(() => {
    throw new Error(`cargo ndk did not produce expected Android library: ${destination}`);
  });
  await stripAndroidSharedLibraries(outputDir, env.ANDROID_NDK_HOME);
  console.log(`Built Android library ${destination}`);

  await run("gradle", ["-p", "packages/raster-android", ":raster-android:assembleRelease"], {
    env,
  });
  const aarPath = path.join(
    rootDir,
    "packages/raster-android/raster-android/build/outputs/aar/raster-android-release.aar",
  );
  await access(aarPath).catch(() => {
    throw new Error(`Gradle did not produce expected Android AAR: ${aarPath}`);
  });
  console.log(`Built Android AAR ${aarPath}`);
}

async function stripAndroidSharedLibraries(jniLibsDir, ndkRoot) {
  const strip = await resolveAndroidLlvmTool(ndkRoot, "llvm-strip");
  const libraries = await findFiles(jniLibsDir, (filePath) => filePath.endsWith(".so"));
  for (const library of libraries) {
    await run(strip, ["--strip-unneeded", library]);
    await chmod(library, 0o755);
    console.log(`Stripped Android library ${library}`);
  }
}

async function resolveAndroidLlvmTool(ndkRoot, toolName) {
  const prebuiltDir = path.join(ndkRoot, "toolchains", "llvm", "prebuilt");
  const hosts = (await readdir(prebuiltDir, { withFileTypes: true }))
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
  for (const host of hosts) {
    const toolPath = path.join(prebuiltDir, host, "bin", toolName);
    try {
      await access(toolPath);
      return toolPath;
    } catch {
      // Try the next prebuilt host directory.
    }
  }
  throw new Error(`Android NDK tool not found: ${toolName}`);
}

async function findFiles(directory, predicate) {
  const results = [];
  const entries = await readdir(directory, { withFileTypes: true });
  for (const entry of entries) {
    const filePath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      results.push(...await findFiles(filePath, predicate));
    } else if (entry.isFile() && predicate(filePath)) {
      results.push(filePath);
    }
  }
  return results;
}

async function androidBuildEnv() {
  const sdkRoot = await resolveAndroidSdkRoot();
  const ndkRoot = await resolveAndroidNdkRoot(sdkRoot);
  return {
    ANDROID_HOME: sdkRoot,
    ANDROID_SDK_ROOT: sdkRoot,
    ANDROID_NDK_HOME: ndkRoot,
    ANDROID_NDK_ROOT: ndkRoot,
    CMAKE_ANDROID_NDK: ndkRoot,
    CMAKE_GENERATOR: "Ninja",
  };
}

async function cleanAndroidCmakeCaches(platform) {
  const buildRoot = path.join(rootDir, "target", platform.rustTarget, "release", "build");
  let entries;
  try {
    entries = await readdir(buildRoot, { withFileTypes: true });
  } catch {
    return;
  }

  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }
    const buildDirectory = path.join(buildRoot, entry.name, "out", "build");
    try {
      await access(path.join(buildDirectory, "CMakeCache.txt"));
    } catch {
      continue;
    }
    await rm(buildDirectory, { recursive: true, force: true });
  }
}

async function resolveAndroidSdkRoot() {
  const configured = process.env.ANDROID_HOME || process.env.ANDROID_SDK_ROOT;
  if (configured) {
    await access(configured);
    return configured;
  }

  const defaultRoot = path.join(process.env.HOME ?? "", "Library", "Android", "sdk");
  await access(defaultRoot).catch(() => {
    throw new Error("Android SDK not found. Set ANDROID_HOME or ANDROID_SDK_ROOT.");
  });
  return defaultRoot;
}

async function resolveAndroidNdkRoot(sdkRoot) {
  const configured = process.env.ANDROID_NDK_HOME || process.env.ANDROID_NDK_ROOT;
  if (configured) {
    await access(configured);
    return configured;
  }

  const ndkDirectory = path.join(sdkRoot, "ndk");
  const versions = (await readdir(ndkDirectory, { withFileTypes: true }))
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort(compareVersionNames);
  const version = versions.at(-1);
  if (!version) {
    throw new Error(`Android NDK not found under ${ndkDirectory}`);
  }
  return path.join(ndkDirectory, version);
}

function compareVersionNames(left, right) {
  const leftParts = left.split(".").map((part) => Number(part));
  const rightParts = right.split(".").map((part) => Number(part));
  const length = Math.max(leftParts.length, rightParts.length);
  for (let index = 0; index < length; index += 1) {
    const diff = (leftParts[index] || 0) - (rightParts[index] || 0);
    if (diff !== 0) {
      return diff;
    }
  }
  return left.localeCompare(right);
}

async function writeBinaryPackageManifest(platform) {
  const packageDir = path.join(rootDir, platform.packageDir);
  const packagePath = path.join(packageDir, "package.json");
  const version = await readReleaseVersion();
  await mkdir(packageDir, { recursive: true });
  await writeJson(packagePath, {
    name: `raster-bin-${platform.id}`,
    version,
    bin: {
      raster: platform.packageBinary,
    },
    os: [platform.os],
    cpu: [platform.cpu],
    libc: ["any"],
    files: ["bin"],
    publishConfig: {
      access: "public",
    },
  });
}

async function readReleaseVersion() {
  const config = JSON.parse(await readFile(configPath, "utf8"));
  const version = config.version;
  if (typeof version !== "string" || version.trim() === "") {
    throw new Error("config.json must define a non-empty string version");
  }
  return version;
}

async function updatePackageSwiftChecksum(version, checksum) {
  const packagePath = path.join(rootDir, "Package.swift");
  const source = await readFile(packagePath, "utf8");
  const url = `https://github.com/Ray-D-Song/raster/releases/download/v${version}/RasterRuntime.xcframework.zip`;
  let next = source.replace(
    /url: "https:\/\/github\.com\/Ray-D-Song\/raster\/releases\/download\/v[^"]+\/RasterRuntime\.xcframework\.zip"/,
    `url: "${url}"`,
  );
  next = next.replace(
    /checksum: "[0-9a-f]{64}"/,
    `checksum: "${checksum}"`,
  );
  if (next === source) {
    throw new Error("Package.swift did not contain the expected RasterRuntime binary target URL/checksum");
  }
  await writeFile(packagePath, next);
}

async function writeJson(filePath, value) {
  await writeFile(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

async function targetBuildEnv(platform) {
  const env = {};
  const target = platform.rustTarget.replaceAll("-", "_");
  const cargoTarget = target.toUpperCase();
  if (platform.linker) {
    env[`CARGO_TARGET_${cargoTarget}_LINKER`] = platform.linker;
  }
  if (platform.cc) {
    env[`CC_${target}`] = platform.cc;
  }
  if (platform.cxx) {
    env[`CXX_${target}`] = platform.cxx;
  }
  if (platform.ar) {
    env[`AR_${target}`] = platform.ar;
    env[`CMAKE_AR_${target}`] = platform.ar;
  }
  if (platform.ranlib) {
    env[`RANLIB_${target}`] = platform.ranlib;
  }
  if (platform.cmakeSystemName) {
    const toolchainFile = await writeCmakeToolchainFile(platform);
    env[`CMAKE_TOOLCHAIN_FILE_${target}`] = toolchainFile;
    env.TARGET_CMAKE_TOOLCHAIN_FILE = toolchainFile;
    env.CMAKE_TOOLCHAIN_FILE = toolchainFile;
  }
  return env;
}

async function writeCmakeToolchainFile(platform) {
  const directory = path.join(rootDir, "target", "raster-cross-toolchains");
  const file = path.join(directory, `${platform.id}.cmake`);
  await mkdir(directory, { recursive: true });
  const lines = [
    `set(CMAKE_SYSTEM_NAME ${platform.cmakeSystemName})`,
    `set(CMAKE_SYSTEM_PROCESSOR ${platform.cmakeSystemProcessor})`,
  ];
  await pushCmakeTool(platform, lines, "CMAKE_C_COMPILER", platform.cc);
  await pushCmakeTool(platform, lines, "CMAKE_CXX_COMPILER", platform.cxx);
  await pushCmakeTool(platform, lines, "CMAKE_AR", platform.ar);
  await pushCmakeTool(platform, lines, "CMAKE_RANLIB", platform.ranlib);
  await writeFile(file, `${lines.join("\n")}\n`, "utf8");
  return file;
}

async function pushCmakeTool(platform, lines, name, command) {
  if (!command) {
    return;
  }
  lines.push(`set(${name} ${cmakeQuote(await resolveCommand(command))} CACHE FILEPATH "" FORCE)`);
}

async function resolveCommand(command) {
  const lookup = process.platform === "win32" ? "where" : "sh";
  const args = process.platform === "win32" ? [command] : ["-c", `command -v ${shellQuote(command)}`];
  const stdout = await capture(lookup, args);
  return stdout.split(/\r?\n/).map((line) => line.trim()).find(Boolean) ?? command;
}

function cmakeQuote(value) {
  return `"${value.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"`;
}

function capture(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd ?? rootDir,
      stdio: ["ignore", "pipe", "inherit"],
      shell: options.shell ?? process.platform === "win32",
    });
    let stdout = "";
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0) {
        resolve(stdout);
        return;
      }
      const suffix = signal ? `signal ${signal}` : `exit code ${code}`;
      reject(new Error(`${command} ${args.join(" ")} failed with ${suffix}`));
    });
  });
}

function run(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd ?? rootDir,
      env: { ...process.env, ...options.env },
      stdio: "inherit",
      shell: process.platform === "win32",
    });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0) {
        resolve();
        return;
      }
      const suffix = signal ? `signal ${signal}` : `exit code ${code}`;
      reject(new Error(`${command} ${args.join(" ")} failed with ${suffix}`));
    });
  });
}

main(process.argv.slice(2)).catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
