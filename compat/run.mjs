import { spawn } from "node:child_process";
import fs from "node:fs/promises";
import net from "node:net";
import path from "node:path";

const HTTP_CHECK_TIMEOUT_MS = 5_000;
const BUILD_TIMEOUT_MS = 120_000;
const READINESS_TIMEOUT_MS = 30_000;
const READINESS_REQUEST_TIMEOUT_MS = 2_000;
const SERVER_STOP_TIMEOUT_MS = 5_000;

const [name, rasterPath] = process.argv.slice(2);
const root = process.cwd();
const cases = {
  next: {
    directory: "compat/next",
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
      [".vite", "manifest.json"],
    ],
  },
};

const testCase = cases[name];
if (!testCase || !rasterPath) {
  throw new Error(
    "Usage: node compat/run.mjs <next|vite-plus> <raster-runtime>"
  );
}

const directory = path.join(root, testCase.directory);
const raster = path.resolve(root, rasterPath);
const logPath = path.join(directory, "compat.log");

if (name === "next") {
  await runNextStandalone(directory, raster, logPath, root);
} else {
  await runVitePlusBuild(testCase, directory, raster, logPath, root);
}

async function runVitePlusBuild(testCase, directory, raster, logPath, root) {
  const output = path.join(directory, testCase.output);
  const command = path.join(directory, testCase.command);

  await fs.rm(output, { recursive: true, force: true });

  const result = await spawnCollect(
    raster,
    [command, ...testCase.args],
    {
      cwd: directory,
      env: { ...process.env },
    },
    BUILD_TIMEOUT_MS
  );

  const timedOut = result.timedOut ? " (timed out)" : "";
  const log =
    `$ ${raster} ${command} ${testCase.args.join(" ")}\n\n` +
    `exit: ${result.code ?? result.signal}${timedOut}\n\n` +
    `stdout:\n${result.stdout}\n\nstderr:\n${result.stderr}\n`;
  await fs.writeFile(logPath, log);
  process.stdout.write(result.stdout);
  process.stderr.write(result.stderr);

  const outputExists = await pathExists(output);

  if (result.timedOut) {
    throw new Error(
      `${name} build timed out after ${BUILD_TIMEOUT_MS}ms. ` +
        `See ${path.relative(root, logPath)} for captured output.`
    );
  }

  if (result.code !== 0) {
    throw new Error(
      `${name} build exited with ${result.code ?? result.signal}`
    );
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

  const [esm, cjs, css, manifest] = await Promise.all([
    fs.readFile(path.join(output, "index.js"), "utf8"),
    fs.readFile(path.join(output, "index.cjs"), "utf8"),
    fs.readFile(path.join(output, "style.css"), "utf8"),
    fs.readFile(path.join(output, ".vite", "manifest.json"), "utf8"),
  ]);
  if (
    !esm.includes("Button") ||
    !cjs.includes("Button") ||
    !css.includes(".raster-button") ||
    !manifest.includes("src/index.tsx")
  ) {
    throw new Error(
      "Vite+ build output is missing an expected library artifact"
    );
  }

  console.log(`${name} compatibility build passed`);
}

async function runNextStandalone(directory, raster, logPath, root) {
  const logParts = [];
  const outputDir = path.join(directory, ".next");
  const standaloneDir = path.join(outputDir, "standalone");
  const serverEntry = path.join(standaloneDir, "server.js");
  const nextCli = path.join(directory, "node_modules/next/dist/bin/next");

  await fs.rm(outputDir, { recursive: true, force: true });

  // --- Phase 1: Node builds standalone (not Raster) ---
  const buildCmd = `${process.execPath} ${nextCli} build`;
  logParts.push(`# Node build\n$ ${buildCmd}`);
  console.log(`[compat-next] building with system Node: ${buildCmd}`);

  const buildResult = await spawnCollect(
    process.execPath,
    [nextCli, "build"],
    {
      cwd: directory,
      env: {
        ...process.env,
        NEXT_TELEMETRY_DISABLED: "1",
        NODE_ENV: "production",
      },
    },
    BUILD_TIMEOUT_MS
  );

  const buildExitLabel = buildResult.timedOut
    ? `timeout after ${BUILD_TIMEOUT_MS}ms`
    : String(buildResult.code ?? buildResult.signal);
  logParts.push(
    `exit: ${buildExitLabel}\n\nstdout:\n${buildResult.stdout}\n\nstderr:\n${buildResult.stderr}`
  );
  process.stdout.write(buildResult.stdout);
  process.stderr.write(buildResult.stderr);

  if (buildResult.timedOut) {
    await writeLog(logPath, logParts);
    throw new Error(
      `Next Node build timed out after ${BUILD_TIMEOUT_MS}ms. ` +
        `See ${path.relative(root, logPath)} for Node build stdout/stderr. ` +
        `Raster was not started.`
    );
  }

  if (buildResult.code !== 0) {
    await writeLog(logPath, logParts);
    throw new Error(
      `Next Node build failed (exit ${buildResult.code ?? buildResult.signal}). ` +
        `See ${path.relative(root, logPath)} for Node build stdout/stderr. ` +
        `Raster was not started.`
    );
  }

  if (!(await pathExists(serverEntry))) {
    await writeLog(logPath, logParts);
    throw new Error(
      `Next Node build succeeded but missing standalone entry: ${path.relative(root, serverEntry)}. ` +
        `Ensure next.config has output: "standalone". See ${path.relative(root, logPath)}.`
    );
  }

  logParts.push(
    `\n# Standalone entry present\n${path.relative(root, serverEntry)}`
  );

  // --- Phase 2: Raster runs standalone server ---
  const port = await getFreePort();
  const host = "127.0.0.1";
  const baseUrl = `http://${host}:${port}`;
  const serverEnv = {
    ...process.env,
    HOSTNAME: host,
    PORT: String(port),
    NODE_ENV: "production",
    NEXT_TELEMETRY_DISABLED: "1",
  };

  const startCmd = `${raster} ${serverEntry}`;
  logParts.push(
    `\n# Raster start\n$ ${startCmd}\n` +
      `cwd: ${standaloneDir}\n` +
      `HOSTNAME=${host} PORT=${port} NODE_ENV=production NEXT_TELEMETRY_DISABLED=1`
  );
  console.log(`[compat-next] starting with Raster on ${baseUrl}`);

  let server = null;
  let serverExit = null;
  let stdout = "";
  let stderr = "";

  try {
    server = spawn(raster, [serverEntry], {
      cwd: standaloneDir,
      env: serverEnv,
      stdio: ["ignore", "pipe", "pipe"],
    });

    server.stdout.on("data", (chunk) => {
      stdout += chunk;
      process.stdout.write(chunk);
    });
    server.stderr.on("data", (chunk) => {
      stderr += chunk;
      process.stderr.write(chunk);
    });
    server.on("error", (err) => {
      serverExit = { code: null, signal: null, error: err };
    });
    server.on("close", (code, signal) => {
      if (!serverExit) {
        serverExit = { code, signal, error: null };
      }
    });

    // Wait for readiness via health endpoint (not console text)
    const ready = await waitForReady({
      url: `${baseUrl}/api/health`,
      timeoutMs: READINESS_TIMEOUT_MS,
      isExited: () => serverExit !== null,
      getExit: () => serverExit,
    });

    if (!ready.ok) {
      const lastErrorLine = ready.lastError
        ? `last readiness error: ${ready.lastError}\n`
        : "last readiness error: (none recorded)\n";
      logParts.push(
        `\n# Raster early exit / readiness\n` +
          `reason: ${ready.reason}\n` +
          lastErrorLine +
          `exit: ${formatExit(serverExit)}\n\n` +
          `stdout:\n${stdout}\n\nstderr:\n${stderr}`
      );
      await writeLog(logPath, logParts);
      if (ready.reason === "exited") {
        throw new Error(
          `Raster exited before listening (${formatExit(serverExit)}). ` +
            (ready.lastError
              ? `Last readiness error: ${ready.lastError}. `
              : "") +
            `See ${path.relative(root, logPath)} for Raster stdout/stderr.`
        );
      }
      throw new Error(
        `Raster standalone server readiness timed out after ${READINESS_TIMEOUT_MS / 1000}s ` +
          `(health ${baseUrl}/api/health). ` +
          (ready.lastError
            ? `Last readiness error: ${ready.lastError}. `
            : "") +
          `See ${path.relative(root, logPath)} for Raster stdout/stderr.`
      );
    }

    logParts.push(
      `\n# Raster ready\nhealth: ${baseUrl}/api/health OK\n\nstdout so far:\n${stdout}\n\nstderr so far:\n${stderr}`
    );

    // --- Phase 3: HTTP assertions ---
    const checks = [
      {
        name: "GET /",
        path: "/",
        expectStatus: 200,
        assert: (body) => {
          if (!body.includes("Raster Next compatibility fixture")) {
            return 'body missing "Raster Next compatibility fixture"';
          }
          return null;
        },
      },
      {
        name: "GET /api/health",
        path: "/api/health",
        expectStatus: 200,
        assert: (body) => {
          let json;
          try {
            json = JSON.parse(body);
          } catch {
            return `invalid JSON: ${truncate(body, 200)}`;
          }
          if (json?.status !== "ok") {
            return `expected { "status": "ok" }, got ${JSON.stringify(json)}`;
          }
          return null;
        },
      },
      {
        name: "GET /posts/42",
        path: "/posts/42",
        expectStatus: 200,
        assert: (body) => {
          // Next SSR may insert HTML comments between text nodes ("Post <!-- -->42").
          const normalized = body.replace(/<!--[\s\S]*?-->/g, "");
          if (!normalized.includes("Post 42")) {
            return 'body missing "Post 42" (after stripping HTML comments)';
          }
          return null;
        },
      },
    ];

    const failures = [];
    for (const check of checks) {
      if (serverExit) {
        failures.push(
          `${check.name}: skipped (Raster already exited: ${formatExit(serverExit)})`
        );
        logParts.push(
          `\n# HTTP ${check.name}\nskipped: Raster exited ${formatExit(serverExit)}`
        );
        continue;
      }

      try {
        // One AbortSignal covers both fetch headers and body consumption.
        const signal = AbortSignal.timeout(HTTP_CHECK_TIMEOUT_MS);
        const res = await fetch(`${baseUrl}${check.path}`, { signal });
        const body = await res.text();
        const statusOk = res.status === check.expectStatus;
        const assertMsg = statusOk ? check.assert(body) : null;
        const ok = statusOk && !assertMsg;

        logParts.push(
          `\n# HTTP ${check.name}\n` +
            `url: ${baseUrl}${check.path}\n` +
            `status: ${res.status} (expected ${check.expectStatus})\n` +
            `ok: ${ok}\n` +
            (assertMsg ? `assert: ${assertMsg}\n` : "") +
            `body:\n${truncate(body, 2000)}`
        );

        if (!statusOk) {
          failures.push(
            `${check.name}: expected status ${check.expectStatus}, got ${res.status}`
          );
        } else if (assertMsg) {
          failures.push(`${check.name}: ${assertMsg}`);
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        logParts.push(`\n# HTTP ${check.name}\nerror: ${msg}`);
        failures.push(`${check.name}: request failed: ${msg}`);
      }
    }

    // Capture any additional server output after requests
    logParts.push(
      `\n# Raster output after HTTP checks\nstdout:\n${stdout}\n\nstderr:\n${stderr}\n` +
        `exit so far: ${formatExit(serverExit)}`
    );

    await writeLog(logPath, logParts);

    if (failures.length > 0) {
      throw new Error(
        `Next standalone runtime checks failed:\n  - ${failures.join("\n  - ")}\n` +
          `See ${path.relative(root, logPath)} for full diagnostics.`
      );
    }

    console.log(
      "next compatibility standalone runtime passed " +
        "(Node build + Raster run; HTTP / /api/health /posts/42 OK)"
    );
  } finally {
    if (server) {
      await stopProcess(server, SERVER_STOP_TIMEOUT_MS);
      logParts.push(
        `\n# Server cleanup\nsent SIGTERM then SIGKILL if needed; final exit: ${formatExit(serverExit)}`
      );
      try {
        await writeLog(logPath, logParts);
      } catch {
        // ignore secondary log write failures during cleanup
      }
    }
  }
}

/**
 * Spawn a process, collect stdout/stderr, optionally enforce a wall-clock timeout.
 * On timeout: SIGTERM, then SIGKILL after a short grace period.
 * Returns { code, signal, stdout, stderr, timedOut }.
 */
function spawnCollect(command, args, options, timeoutMs = 0) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      ...options,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    let timedOut = false;
    let settled = false;
    let killTimer = null;

    const finish = (code, signal) => {
      if (settled) return;
      settled = true;
      if (timer) clearTimeout(timer);
      if (killTimer) clearTimeout(killTimer);
      resolve({ code, signal, stdout, stderr, timedOut });
    };

    child.stdout.on("data", (chunk) => (stdout += chunk));
    child.stderr.on("data", (chunk) => (stderr += chunk));
    child.on("error", (err) => {
      if (settled) return;
      settled = true;
      if (timer) clearTimeout(timer);
      if (killTimer) clearTimeout(killTimer);
      reject(err);
    });
    child.on("close", (code, signal) => finish(code, signal));

    let timer = null;
    if (timeoutMs > 0) {
      timer = setTimeout(() => {
        timedOut = true;
        try {
          child.kill("SIGTERM");
        } catch {
          // ignore
        }
        killTimer = setTimeout(() => {
          try {
            if (child.exitCode === null && child.signalCode === null) {
              child.kill("SIGKILL");
            }
          } catch {
            // ignore
          }
        }, 2_000);
      }, timeoutMs);
    }
  });
}

function getFreePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close((err) => {
        if (err) reject(err);
        else resolve(port);
      });
    });
    server.on("error", reject);
  });
}

async function waitForReady({ url, timeoutMs, isExited, getExit }) {
  const deadline = Date.now() + timeoutMs;
  let lastError = null;

  while (Date.now() < deadline) {
    if (isExited()) {
      return { ok: false, reason: "exited", exit: getExit(), lastError };
    }
    try {
      const res = await fetch(url, {
        signal: AbortSignal.timeout(READINESS_REQUEST_TIMEOUT_MS),
      });
      if (res.ok) {
        return { ok: true };
      }
      lastError = `status ${res.status}`;
    } catch (err) {
      lastError = err instanceof Error ? err.message : String(err);
    }
    await sleep(250);
  }

  if (isExited()) {
    return { ok: false, reason: "exited", exit: getExit(), lastError };
  }
  return { ok: false, reason: "timeout", lastError };
}

function stopProcess(child, timeoutMs) {
  return new Promise((resolve) => {
    if (child.exitCode !== null || child.signalCode !== null) {
      resolve();
      return;
    }

    let settled = false;
    const done = () => {
      if (settled) return;
      settled = true;
      resolve();
    };

    child.once("close", done);

    try {
      child.kill("SIGTERM");
    } catch {
      done();
      return;
    }

    const timer = setTimeout(() => {
      try {
        if (child.exitCode === null && child.signalCode === null) {
          child.kill("SIGKILL");
        }
      } catch {
        // ignore
      }
      // Give SIGKILL a moment, then resolve either way
      setTimeout(done, 500);
    }, timeoutMs);

    child.once("close", () => clearTimeout(timer));
  });
}

async function pathExists(p) {
  try {
    await fs.access(p);
    return true;
  } catch {
    return false;
  }
}

async function writeLog(logPath, parts) {
  await fs.writeFile(logPath, parts.join("\n") + "\n");
}

function formatExit(exit) {
  if (!exit) return "still running";
  if (exit.error) return `spawn error: ${exit.error.message}`;
  if (exit.signal) return `signal ${exit.signal}`;
  return `code ${exit.code}`;
}

function truncate(text, max) {
  if (text.length <= max) return text;
  return text.slice(0, max) + `\n... (${text.length - max} more bytes)`;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
