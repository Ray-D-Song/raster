#!/usr/bin/env node

import { access, chmod, copyFile, mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const rootDir = path.resolve(path.dirname(__filename), "..");

const platforms = [
  {
    id: "darwin-arm64",
    rustTarget: "aarch64-apple-darwin",
    packageDir: "packages/raster-bin-darwin-arm64",
    packageBinary: "bin/raster",
    cargoBinary: "raster",
    tools: [],
  },
  {
    id: "darwin-x64",
    rustTarget: "x86_64-apple-darwin",
    packageDir: "packages/raster-bin-darwin-x64",
    packageBinary: "bin/raster",
    cargoBinary: "raster",
    tools: [],
  },
  {
    id: "linux-arm64",
    rustTarget: "aarch64-unknown-linux-gnu",
    packageDir: "packages/raster-bin-linux-arm64",
    packageBinary: "bin/raster",
    cargoBinary: "raster",
    linker: "aarch64-linux-gnu-gcc",
    cc: "aarch64-linux-gnu-gcc",
    cxx: "aarch64-linux-gnu-g++",
    ar: "aarch64-linux-gnu-ar",
    ranlib: "aarch64-linux-gnu-ranlib",
    cmakeSystemName: "Linux",
    cmakeSystemProcessor: "aarch64",
    rustcLinkArgs: [
      "-C",
      "link-arg=-Wl,-Bstatic",
      "-C",
      "link-arg=-lz-ng",
      "-C",
      "link-arg=-Wl,-Bdynamic",
    ],
    tools: [
      {
        command: "aarch64-linux-gnu-gcc",
        install: {
          darwin: brewFormula("messense/macos-cross-toolchains/aarch64-unknown-linux-gnu"),
        },
      },
      {
        command: "aarch64-linux-gnu-g++",
        install: {
          darwin: brewFormula("messense/macos-cross-toolchains/aarch64-unknown-linux-gnu"),
        },
      },
      {
        command: "aarch64-linux-gnu-ar",
        install: {
          darwin: brewFormula("messense/macos-cross-toolchains/aarch64-unknown-linux-gnu"),
        },
      },
      {
        command: "aarch64-linux-gnu-ranlib",
        install: {
          darwin: brewFormula("messense/macos-cross-toolchains/aarch64-unknown-linux-gnu"),
        },
      },
      {
        command: "cmake",
        install: {
          darwin: brewFormula("cmake"),
        },
      },
    ],
  },
  {
    id: "linux-x64",
    rustTarget: "x86_64-unknown-linux-gnu",
    packageDir: "packages/raster-bin-linux-x64",
    packageBinary: "bin/raster",
    cargoBinary: "raster",
    linker: "x86_64-linux-gnu-gcc",
    cc: "x86_64-linux-gnu-gcc",
    cxx: "x86_64-linux-gnu-g++",
    ar: "x86_64-linux-gnu-ar",
    ranlib: "x86_64-linux-gnu-ranlib",
    cmakeSystemName: "Linux",
    cmakeSystemProcessor: "x86_64",
    rustcLinkArgs: [
      "-C",
      "link-arg=-Wl,-Bstatic",
      "-C",
      "link-arg=-lz-ng",
      "-C",
      "link-arg=-Wl,-Bdynamic",
    ],
    tools: [
      {
        command: "x86_64-linux-gnu-gcc",
        install: {
          darwin: brewFormula("messense/macos-cross-toolchains/x86_64-unknown-linux-gnu"),
        },
      },
      {
        command: "x86_64-linux-gnu-g++",
        install: {
          darwin: brewFormula("messense/macos-cross-toolchains/x86_64-unknown-linux-gnu"),
        },
      },
      {
        command: "x86_64-linux-gnu-ar",
        install: {
          darwin: brewFormula("messense/macos-cross-toolchains/x86_64-unknown-linux-gnu"),
        },
      },
      {
        command: "x86_64-linux-gnu-ranlib",
        install: {
          darwin: brewFormula("messense/macos-cross-toolchains/x86_64-unknown-linux-gnu"),
        },
      },
      {
        command: "cmake",
        install: {
          darwin: brewFormula("cmake"),
        },
      },
    ],
  },
  {
    id: "win32-x64",
    rustTarget: "x86_64-pc-windows-gnu",
    packageDir: "packages/raster-bin-win32-x64",
    packageBinary: "bin/raster.exe",
    cargoBinary: "raster.exe",
    linker: "x86_64-w64-mingw32-gcc",
    cc: "x86_64-w64-mingw32-gcc",
    cxx: "x86_64-w64-mingw32-g++",
    ar: "x86_64-w64-mingw32-ar",
    ranlib: "x86_64-w64-mingw32-ranlib",
    cmakeSystemName: "Windows",
    cmakeSystemProcessor: "x86_64",
    tools: [
      {
        command: "x86_64-w64-mingw32-gcc",
        install: {
          darwin: brewFormula("mingw-w64"),
        },
      },
      {
        command: "x86_64-w64-mingw32-g++",
        install: {
          darwin: brewFormula("mingw-w64"),
        },
      },
      {
        command: "x86_64-w64-mingw32-ar",
        install: {
          darwin: brewFormula("mingw-w64"),
        },
      },
      {
        command: "x86_64-w64-mingw32-ranlib",
        install: {
          darwin: brewFormula("mingw-w64"),
        },
      },
      {
        command: "cmake",
        install: {
          darwin: brewFormula("cmake"),
        },
      },
    ],
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
      console.log(`${platform.id}\t${platform.rustTarget}`);
    }
    return;
  }

  const selected = args.platforms.length > 0
    ? platforms.filter((platform) => args.platforms.includes(platform.id))
    : platforms;
  const missing = args.platforms.filter(
    (id) => !platforms.some((platform) => platform.id === id),
  );
  if (missing.length > 0) {
    throw new Error(`Unknown platform(s): ${missing.join(", ")}`);
  }

  for (const platform of selected) {
    validatePlatformHost(platform);
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
      throw new Error(`Unknown build-all-platform argument: ${arg}`);
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
  if (platform.id === "win32-x64" && process.platform !== "win32") {
    throw new Error([
      "win32-x64 cannot be built from this host yet.",
      "The current GPUI Windows backend compiles release HLSL shaders in gpui_windows/build.rs behind cfg(target_os = \"windows\").",
      "When cross-compiling from macOS/Linux, that build script does not generate OUT_DIR/shaders_bytes.rs, so the build fails later in directx_renderer.rs.",
      "Build win32-x64 on a Windows host with the Windows SDK fxc.exe available, or patch/fork gpui_windows to support cross-host shader generation.",
    ].join(" "));
  }
}

async function ensureRustTargets(selected) {
  const installed = new Set((await capture("rustup", ["target", "list", "--installed"]))
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean));
  const missing = selected
    .map((platform) => platform.rustTarget)
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
    if (!install) {
      throw new Error(
        `Missing required tool ${tool.command}, and no installer is configured for ${process.platform}`,
      );
    }

    await runInstallStep(install, `Installing missing tool ${tool.command}`);
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

  await mkdir(path.dirname(destination), { recursive: true });
  await rm(destination, { force: true });
  await copyFile(source, destination);
  if (platform.packageBinary.endsWith("/raster")) {
    await chmod(destination, 0o755);
  }
  console.log(`Copied ${source} -> ${destination}`);
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
      cwd: rootDir,
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
      cwd: rootDir,
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
