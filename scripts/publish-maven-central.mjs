#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const rootDir = path.resolve(path.dirname(__filename), "..");
const defaultBundlePath = path.join(
  rootDir,
  "packages/raster-android/build/raster-android-maven-central-bundle.zip",
);
const uploadUrl = "https://central.sonatype.com/api/v1/publisher/upload";
const statusUrl = "https://central.sonatype.com/api/v1/publisher/status";
const deploymentUrl = "https://central.sonatype.com/api/v1/publisher/deployment";

async function main(argv) {
  const args = parseArgs(argv);
  const bundlePath = path.resolve(rootDir, args.bundle);
  const token = authToken();
  const deploymentId = await uploadBundle(bundlePath, token, args.name, args.automatic);
  console.log(`Maven Central deployment uploaded: ${deploymentId}`);
  await waitForDeployment(deploymentId, token);
}

function parseArgs(argv) {
  const args = {
    bundle: defaultBundlePath,
    name: undefined,
    automatic: true,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--bundle") {
      args.bundle = requiredValue(argv[++index], "--bundle");
    } else if (arg.startsWith("--bundle=")) {
      args.bundle = requiredValue(arg.slice("--bundle=".length), "--bundle");
    } else if (arg === "--name") {
      args.name = requiredValue(argv[++index], "--name");
    } else if (arg.startsWith("--name=")) {
      args.name = requiredValue(arg.slice("--name=".length), "--name");
    } else if (arg === "--manual") {
      args.automatic = false;
    } else {
      throw new Error(`Unknown Maven Central publish argument: ${arg}`);
    }
  }

  return args;
}

function requiredValue(value, flag) {
  if (!value) {
    throw new Error(`${flag} requires a value`);
  }
  return value;
}

function authToken() {
  const username = process.env.MAVEN_CENTRAL_USERNAME;
  const password = process.env.MAVEN_CENTRAL_PASSWORD;
  if (!username || !password) {
    throw new Error("MAVEN_CENTRAL_USERNAME and MAVEN_CENTRAL_PASSWORD are required");
  }
  return Buffer.from(`${username}:${password}`, "utf8").toString("base64");
}

async function uploadBundle(bundlePath, token, name, automatic) {
  const bundle = await readFile(bundlePath);
  const formData = new FormData();
  formData.set(
    "bundle",
    new Blob([bundle], { type: "application/octet-stream" }),
    path.basename(bundlePath),
  );

  const url = new URL(uploadUrl);
  url.searchParams.set("publishingType", automatic ? "AUTOMATIC" : "USER_MANAGED");
  if (name) {
    url.searchParams.set("name", name);
  }

  const response = await fetch(url, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
    },
    body: formData,
  });
  const body = await response.text();
  if (!response.ok) {
    throw new Error(`Maven Central upload failed (${response.status}): ${body}`);
  }
  return body.trim();
}

async function waitForDeployment(deploymentId, token) {
  const started = Date.now();
  const timeoutMs = 20 * 60 * 1000;
  let lastState = "";

  while (Date.now() - started < timeoutMs) {
    const status = await deploymentStatus(deploymentId, token);
    const state = status.deploymentState ?? status.state ?? "UNKNOWN";
    if (state !== lastState) {
      console.log(`Maven Central deployment state: ${state}`);
      lastState = state;
    }

    if (state === "PUBLISHED") {
      return;
    }
    if (state === "VALIDATED") {
      await publishValidatedDeployment(deploymentId, token);
    } else if (state === "FAILED") {
      throw new Error(`Maven Central deployment failed: ${JSON.stringify(status)}`);
    }

    await delay(10_000);
  }

  throw new Error(`Timed out waiting for Maven Central deployment ${deploymentId}`);
}

async function deploymentStatus(deploymentId, token) {
  const url = new URL(statusUrl);
  url.searchParams.set("id", deploymentId);
  const response = await fetch(url, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
  const body = await response.text();
  if (!response.ok) {
    throw new Error(`Maven Central status failed (${response.status}): ${body}`);
  }
  return JSON.parse(body);
}

async function publishValidatedDeployment(deploymentId, token) {
  const response = await fetch(`${deploymentUrl}/${deploymentId}`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
  const body = await response.text();
  if (!response.ok) {
    throw new Error(`Maven Central publish failed (${response.status}): ${body}`);
  }
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

main(process.argv.slice(2)).catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
