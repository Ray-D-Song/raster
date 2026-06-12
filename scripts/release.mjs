#!/usr/bin/env node

import { constants } from "node:fs";
import { access, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const rootDir = path.resolve(path.dirname(__filename), "..");
const configPath = path.join(rootDir, "config.json");

const jsTargets = [
  target("raster-js", "packages/raster", "js", ["raster"]),
  target("unplugin-raster", "packages/unplugin-raster", "js", ["plugin"]),
  target("raster-cli", "packages/cli", "js", ["cli"]),
];

const binaryTargets = [
  target("raster-bin-darwin-arm64", "packages/raster-bin-darwin-arm64", "bin", ["darwin-arm64"]),
  target("raster-bin-darwin-x64", "packages/raster-bin-darwin-x64", "bin", ["darwin-x64"]),
  target("raster-bin-linux-arm64", "packages/raster-bin-linux-arm64", "bin", ["linux-arm64"]),
  target("raster-bin-linux-x64", "packages/raster-bin-linux-x64", "bin", ["linux-x64"]),
  target("raster-bin-win32-x64", "packages/raster-bin-win32-x64", "bin", ["win32-x64"]),
];

const allTargets = [...binaryTargets, ...jsTargets];
const versionedPackages = [...jsTargets, ...binaryTargets].map((item) => item.packageDir);
const consumerPackages = ["apps/showcase", "packages/cli/templates/default"];
const targetAliases = buildTargetAliases(allTargets);

async function main(argv) {
  const args = parseArgs(argv);
  const version = await readReleaseVersion();
  const tag = version.includes("alpha") ? "alpha" : "latest";
  const publishTargets = resolvePublishTargets(args);

  await syncVersions(version);
  await run("pnpm", ["install", "--lockfile-only", "--link-workspace-packages=true"]);
  await validateBinaryTargets(publishTargets.filter((item) => item.kind === "bin"));

  for (const publishTarget of publishTargets) {
    await run("npm", ["pack", "--dry-run"], {
      cwd: path.join(rootDir, publishTarget.packageDir),
    });
  }

  if (args.dryRun) {
    console.log(
      `Dry run complete for ${version}. Publish tag would be ${tag}. Packages: ${publishTargets.map((item) => item.name).join(", ")}`,
    );
    return;
  }

  for (const publishTarget of publishTargets) {
    await run(
      "npm",
      [
        "publish",
        "--access",
        "public",
        "--tag",
        tag,
        ...args.otpArgs,
      ],
      { cwd: path.join(rootDir, publishTarget.packageDir) },
    );
  }

  console.log(`Published ${publishTargets.length} package(s) at ${version} with tag ${tag}.`);
}

function target(name, packageDir, kind, aliases = []) {
  return { name, packageDir, kind, aliases: [name, ...aliases] };
}

function buildTargetAliases(targets) {
  const aliases = new Map();
  for (const releaseTarget of targets) {
    for (const alias of releaseTarget.aliases) {
      if (aliases.has(alias)) {
        throw new Error(`Duplicate release target alias: ${alias}`);
      }
      aliases.set(alias, releaseTarget);
    }
  }
  return aliases;
}

function parseArgs(argv) {
  const args = {
    domain: "js",
    dryRun: false,
    include: [],
    exclude: [],
    otpArgs: [],
  };
  let sawDomain = false;

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (isReleaseDomain(arg)) {
      if (sawDomain) {
        throw new Error(`Release domain was provided more than once: ${arg}`);
      }
      args.domain = arg;
      sawDomain = true;
    } else if (arg === "--dry-run") {
      args.dryRun = true;
    } else if (arg === "--include") {
      const value = argv[index + 1];
      if (!value) {
        throw new Error("--include requires a comma-separated value");
      }
      args.include.push(...splitTargets(value, "--include"));
      index += 1;
    } else if (arg.startsWith("--include=")) {
      args.include.push(...splitTargets(arg.slice("--include=".length), "--include"));
    } else if (arg === "--exclude") {
      const value = argv[index + 1];
      if (!value) {
        throw new Error("--exclude requires a comma-separated value");
      }
      args.exclude.push(...splitTargets(value, "--exclude"));
      index += 1;
    } else if (arg.startsWith("--exclude=")) {
      args.exclude.push(...splitTargets(arg.slice("--exclude=".length), "--exclude"));
    } else if (arg === "--otp") {
      const otp = argv[index + 1];
      if (!otp) {
        throw new Error("--otp requires a value");
      }
      args.otpArgs.push("--otp", otp);
      index += 1;
    } else if (arg.startsWith("--otp=")) {
      const otp = arg.slice("--otp=".length);
      if (!otp) {
        throw new Error("--otp requires a value");
      }
      args.otpArgs.push("--otp", otp);
    } else {
      throw new Error(`Unknown release argument: ${arg}`);
    }
  }

  args.include = [...new Set(args.include)];
  args.exclude = [...new Set(args.exclude)];
  return args;
}

function isReleaseDomain(value) {
  return value === "js" || value === "bin" || value === "all";
}

function splitTargets(value, flagName) {
  const targets = value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
  if (targets.length === 0) {
    throw new Error(`${flagName} requires at least one target`);
  }
  return targets;
}

function resolvePublishTargets(args) {
  const domainTargets = targetsForDomain(args.domain);
  const includedTargets = args.include.length > 0
    ? resolveTargetNames(args.include, args.domain, "--include")
    : domainTargets;
  const excludedTargets = resolveTargetNames(args.exclude, args.domain, "--exclude");
  const excludedNames = new Set(excludedTargets.map((item) => item.name));
  const publishTargets = domainTargets.filter(
    (item) => includedTargets.some((included) => included.name === item.name) && !excludedNames.has(item.name),
  );

  if (publishTargets.length === 0) {
    throw new Error("Release target selection is empty after applying --include/--exclude");
  }

  return publishTargets;
}

function targetsForDomain(domain) {
  if (domain === "js") {
    return jsTargets;
  }
  if (domain === "bin") {
    return binaryTargets;
  }
  if (domain === "all") {
    return allTargets;
  }
  throw new Error(`Unknown release domain: ${domain}`);
}

function resolveTargetNames(names, domain, flagName) {
  const domainTargets = targetsForDomain(domain);
  const domainNames = new Set(domainTargets.map((item) => item.name));
  const resolved = [];

  for (const name of names) {
    const releaseTarget = targetAliases.get(name);
    if (!releaseTarget) {
      throw new Error(`${flagName} contains unknown target: ${name}`);
    }
    if (!domainNames.has(releaseTarget.name)) {
      throw new Error(
        `${flagName} target ${name} belongs to ${releaseTarget.kind}, but release domain is ${domain}`,
      );
    }
    resolved.push(releaseTarget);
  }

  return [...new Map(resolved.map((item) => [item.name, item])).values()];
}

async function readReleaseVersion() {
  const config = await readJson(configPath);
  const version = config.version;
  if (typeof version !== "string" || version.trim() === "") {
    throw new Error("config.json must define a non-empty string version");
  }
  if (!isValidSemver(version)) {
    throw new Error(`config.json version is not valid semver: ${version}`);
  }
  return version;
}

async function syncVersions(version) {
  for (const packageDir of versionedPackages) {
    const packagePath = path.join(rootDir, packageDir, "package.json");
    const packageJson = await readJson(packagePath);
    packageJson.version = version;
    if (packageJson.name === "unplugin-raster") {
      packageJson.optionalDependencies = packageJson.optionalDependencies ?? {};
      for (const binaryTarget of binaryTargets) {
        packageJson.optionalDependencies[binaryTarget.name] = version;
      }
    }
    await writeJson(packagePath, packageJson);
  }

  for (const packageDir of consumerPackages) {
    const packagePath = path.join(rootDir, packageDir, "package.json");
    const packageJson = await readJson(packagePath);
    packageJson.dependencies = packageJson.dependencies ?? {};
    packageJson.devDependencies = packageJson.devDependencies ?? {};
    packageJson.dependencies["raster-js"] = version;
    packageJson.devDependencies["unplugin-raster"] = version;
    await writeJson(packagePath, packageJson);
  }
}

async function validateBinaryTargets(targets) {
  for (const binaryTarget of targets) {
    const packageJson = await readJson(path.join(rootDir, binaryTarget.packageDir, "package.json"));
    const binaryRelativePath = packageJson.bin?.raster;
    if (typeof binaryRelativePath !== "string" || binaryRelativePath.trim() === "") {
      throw new Error(`${packageJson.name} must define bin.raster`);
    }
    if (packageJson.name.includes("win32") && path.extname(binaryRelativePath) !== ".exe") {
      throw new Error(`${packageJson.name} bin.raster must point to a .exe file`);
    }

    const binaryPath = path.join(rootDir, binaryTarget.packageDir, binaryRelativePath);
    try {
      await access(binaryPath);
    } catch {
      throw new Error(`${packageJson.name} is missing binary artifact: ${binaryPath}`);
    }

    if (!packageJson.name.includes("win32")) {
      try {
        await access(binaryPath, constants.X_OK);
      } catch {
        throw new Error(`${packageJson.name} binary artifact is not executable: ${binaryPath}`);
      }
    }
  }
}

async function readJson(filePath) {
  return JSON.parse(await readFile(filePath, "utf8"));
}

async function writeJson(filePath, value) {
  await writeFile(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function isValidSemver(version) {
  return /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/.test(
    version,
  );
}

function run(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd ?? rootDir,
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
