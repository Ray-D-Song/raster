import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";

import type { NormalizedRasterPluginOptions } from "./core.ts";

const require = createRequire(import.meta.url);
const VITE_DEV_CHILD_ENV = "RASTER_UNPLUGIN_VITE_DEV_CHILD";

const PLATFORM_PACKAGES: Record<string, string> = {
  "darwin-arm64": "raster-bin-darwin-arm64",
  "darwin-x64": "raster-bin-darwin-x64",
  "linux-arm64": "raster-bin-linux-arm64",
  "linux-x64": "raster-bin-linux-x64",
  "win32-x64": "raster-bin-win32-x64",
};

let devProcess: ChildProcess | undefined;
let viteWatchProcess: ChildProcess | undefined;
let cleanupInstalled = false;

export function buildRasterExecutable(options: NormalizedRasterPluginOptions): Promise<void> {
  if (skipRasterBinary()) {
    return Promise.resolve();
  }
  return runRaster(["build", "--bundle", options.outfile, "--out", options.out], {
    mode: "build",
  });
}

export function startRasterDev(options: NormalizedRasterPluginOptions): void {
  if (skipRasterBinary()) {
    return;
  }
  if (devProcess && !devProcess.killed && devProcess.exitCode == null) {
    return;
  }

  const binary = resolveRasterBinary();
  installDevProcessCleanup();
  devProcess = spawn(binary, ["dev", "--bundle", options.outfile], {
    stdio: "inherit",
  });
  devProcess.on("exit", () => {
    devProcess = undefined;
  });
}

export function startViteBuildWatchForRasterDev(options: NormalizedRasterPluginOptions): void {
  if (skipRasterBinary() || isViteDevChild()) {
    return;
  }
  if (viteWatchProcess && !viteWatchProcess.killed && viteWatchProcess.exitCode == null) {
    return;
  }

  const vite = resolveViteBinary();
  installDevProcessCleanup();
  viteWatchProcess = spawn(vite, ["build", "--watch"], {
    cwd: options.root ?? process.cwd(),
    env: {
      ...process.env,
      [VITE_DEV_CHILD_ENV]: "1",
    },
    stdio: "inherit",
    shell: process.platform === "win32",
  });
  viteWatchProcess.on("exit", () => {
    viteWatchProcess = undefined;
  });
}

export function stopRasterDev(): void {
  if (!devProcess || devProcess.killed || devProcess.exitCode != null) {
    devProcess = undefined;
  } else {
    devProcess.kill();
    devProcess = undefined;
  }

  if (!viteWatchProcess || viteWatchProcess.killed || viteWatchProcess.exitCode != null) {
    viteWatchProcess = undefined;
    return;
  }
  viteWatchProcess.kill();
  viteWatchProcess = undefined;
}

function runRaster(args: string[], context: { mode: "build" }): Promise<void> {
  const binary = resolveRasterBinary();
  return new Promise((resolve, reject) => {
    const child = spawn(binary, args, {
      stdio: "inherit",
    });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0) {
        resolve();
        return;
      }
      const suffix = signal ? `signal ${signal}` : `exit code ${code}`;
      reject(new Error(`[raster] raster ${context.mode} failed with ${suffix}`));
    });
  });
}

function resolveRasterBinary(): string {
  if (process.env.RASTER_UNPLUGIN_BINARY) {
    return process.env.RASTER_UNPLUGIN_BINARY;
  }

  const platformKey = `${process.platform}-${process.arch}`;
  const packageName = PLATFORM_PACKAGES[platformKey];
  if (!packageName) {
    throw new Error(`[raster] unsupported platform for Raster binary: ${platformKey}`);
  }

  let packageJsonPath: string;
  try {
    packageJsonPath = require.resolve(`${packageName}/package.json`);
  } catch {
    throw new Error(
      `[raster] missing Raster binary package for ${platformKey}: install optional dependency ${packageName}`,
    );
  }

  const binaryName = process.platform === "win32" ? "raster.exe" : "raster";
  const binaryPath = path.join(path.dirname(packageJsonPath), "bin", binaryName);
  if (!fs.existsSync(binaryPath)) {
    throw new Error(`[raster] Raster binary not found in ${packageName}: ${binaryPath}`);
  }
  return binaryPath;
}

function resolveViteBinary(): string {
  if (process.env.RASTER_UNPLUGIN_VITE_BIN) {
    return process.env.RASTER_UNPLUGIN_VITE_BIN;
  }
  return require.resolve("vite/bin/vite.js");
}

function skipRasterBinary(): boolean {
  return process.env.RASTER_UNPLUGIN_SKIP_BINARY === "1";
}

export function isViteDevChild(): boolean {
  return process.env[VITE_DEV_CHILD_ENV] === "1";
}

function installDevProcessCleanup(): void {
  if (cleanupInstalled) {
    return;
  }
  cleanupInstalled = true;
  const cleanup = () => stopRasterDev();
  process.once("exit", cleanup);
  process.once("SIGINT", () => {
    cleanup();
    process.exit(130);
  });
  process.once("SIGTERM", () => {
    cleanup();
    process.exit(143);
  });
}
