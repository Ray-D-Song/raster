import { spawn } from "node:child_process";
import fs from "node:fs/promises";
import net from "node:net";
import path from "node:path";

const HTTP_CHECK_TIMEOUT_MS = 5_000;
const BUILD_TIMEOUT_MS = 120_000;
const SCRIPT_TIMEOUT_MS = 60_000;
const NODE_BASELINE_TIMEOUT_MS = 30_000;
const READINESS_TIMEOUT_MS = 30_000;
const READINESS_REQUEST_TIMEOUT_MS = 2_000;
const SERVER_STOP_TIMEOUT_MS = 5_000;
const MYSQL_DOCKER_IMAGE = "mysql:8.4";
const MYSQL_HEALTH_TIMEOUT_MS = 120_000;
const MYSQL_DOCKER_RUN_TIMEOUT_MS = 180_000;
const MYSQL_DOCKER_HOST = "127.0.0.1";

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
  "better-sqlite3": {
    directory: "compat/better-sqlite3",
    script: "test.cjs",
    successMarker: "better-sqlite3 compat OK",
  },
  mysql2: {
    directory: "compat/mysql2",
    script: "test.cjs",
    successMarker: "mysql2 compat OK",
  },
  "v8-hello": {
    directory: "compat/v8-hello",
    buildCommand: "npm run build",
    script: "test.cjs",
    successMarker: "v8-hello compat OK",
  },
  "napi-hello": {
    directory: "compat/napi-hello",
    buildCommand: "yarn build",
    scripts: [
      {
        script: "test.cjs",
        successMarker: "napi-hello compat OK",
        maxDurationMs: 2_000,
        expectCode: 0,
      },
      {
        script: "test-async-work-notimer.cjs",
        successMarker: "async-work-notimer-ok",
        maxDurationMs: 1_000,
        expectCode: 0,
      },
      {
        script: "test-tsfn-unref-fast.cjs",
        maxDurationMs: 500,
        expectCode: 0,
      },
      {
        script: "test-tsfn-unref-timer.cjs",
        successMarker: "timer-tsfn-ok",
        maxDurationMs: 2_000,
        expectCode: 0,
      },
      {
        script: "test-tsfn-refd-hold.cjs",
        expectStillRunning: { checkMs: 400, killAfterMs: 1_500 },
      },
      {
        script: "test-module-exports.cjs",
        successMarker: "module-exports-replace-ok",
        maxDurationMs: 1_000,
        expectCode: 0,
      },
    ],
  },
};

const testCase = cases[name];
if (!testCase || !rasterPath) {
  throw new Error(
    "Usage: node compat/run.mjs <next|vite-plus|better-sqlite3|mysql2|v8-hello|napi-hello> <raster-runtime>"
  );
}

const directory = path.join(root, testCase.directory);
const raster = path.resolve(root, rasterPath);
const logPath = path.join(directory, "compat.log");

if (name === "next") {
  await runNextStandalone(directory, raster, logPath, root);
} else if (
  name === "better-sqlite3" ||
  name === "mysql2" ||
  name === "napi-hello" ||
  name === "v8-hello"
) {
  await runScriptCompat(testCase, directory, raster, logPath, root);
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

async function runScriptCompat(testCase, directory, raster, logPath, root) {
  if (name === "mysql2") {
    await runMysql2ScriptCompat(testCase, directory, raster, logPath, root);
    return;
  }

  const logParts = [];
  const childEnv = { ...process.env };

  try {
    if (testCase.buildCommand && process.env.COMPAT_SKIP_BUILD !== "1") {
      logParts.push(`# Build addon\n$ ${testCase.buildCommand}`);
      console.log(`[compat-${name}] building addon: ${testCase.buildCommand}`);
      const buildResult = await spawnCollect(
        "sh",
        ["-c", testCase.buildCommand],
        { cwd: directory, env: { ...process.env } },
        BUILD_TIMEOUT_MS
      );
      logParts.push(
        `exit: ${buildResult.code ?? buildResult.signal}\n\nstdout:\n${buildResult.stdout}\n\nstderr:\n${buildResult.stderr}`
      );
      if (buildResult.code !== 0) {
        throw new Error(
          `${name} addon build failed (exit ${buildResult.code ?? buildResult.signal}). ` +
            `See ${path.relative(root, logPath)}.`
        );
      }
    }

    const scripts = testCase.scripts ?? [
      {
        script: testCase.script,
        successMarker: testCase.successMarker,
        maxDurationMs: SCRIPT_TIMEOUT_MS,
        expectCode: 0,
      },
    ];

    for (const spec of scripts) {
      await runCompatScript(
        spec,
        directory,
        raster,
        logParts,
        root,
        logPath,
        childEnv
      );
    }

    const compatMode =
      process.env.COMPAT_SKIP_NODE_BASELINE === "1"
        ? "Raster only"
        : "Node baseline + Raster";
    console.log(
      `${name} compatibility passed (${scripts.length} script(s), ${compatMode})`
    );
  } finally {
    await writeLog(logPath, logParts);
  }
}

async function runCompatScript(
  spec,
  directory,
  raster,
  logParts,
  root,
  logPath,
  env
) {
  const {
    script,
    successMarker,
    maxDurationMs = SCRIPT_TIMEOUT_MS,
    expectCode = 0,
    mustNotContainStdout,
    expectStillRunning,
  } = spec;
  const label = `${name}/${script}`;
  const skipNodeBaseline = process.env.COMPAT_SKIP_NODE_BASELINE === "1";

  if (expectStillRunning) {
    const { checkMs = 400, killAfterMs = 1_500 } = expectStillRunning;
    const phases = skipNodeBaseline
      ? [["Raster run", raster, [script]]]
      : [
          ["Node baseline", process.execPath, [script]],
          ["Raster run", raster, [script]],
        ];
    for (const [phase, command, args] of phases) {
      const cmdLine = `${command} ${args.join(" ")}`;
      logParts.push(`\n# ${phase}: ${script} (still-running)\n$ ${cmdLine}`);
      console.log(`[compat-${label}] ${phase} (still-running): ${cmdLine}`);
      const result = await spawnStillRunning(
        command,
        args,
        { cwd: directory, env },
        checkMs,
        killAfterMs
      );
      logParts.push(
        `still alive at ${checkMs}ms, killed after ${killAfterMs}ms\n\nstdout:\n${result.stdout}\n\nstderr:\n${result.stderr}`
      );
      if (!result.stillAliveAtCheck) {
        throw new Error(
          `${label} ${phase} exited before ${checkMs}ms. ` +
            `See ${path.relative(root, logPath)}.`
        );
      }
    }
    return;
  }

  if (!skipNodeBaseline) {
    const nodeCmd = `${process.execPath} ${script}`;
    logParts.push(`\n# Node baseline: ${script}\n$ ${nodeCmd}`);
    console.log(`[compat-${label}] Node baseline: ${nodeCmd}`);

    const nodeResult = await spawnCollect(
      process.execPath,
      [script],
      { cwd: directory, env },
      Math.min(NODE_BASELINE_TIMEOUT_MS, maxDurationMs)
    );

    validateCompatRun(label, "Node baseline", nodeResult, {
      maxDurationMs,
      expectCode,
      successMarker,
      mustNotContainStdout,
      logPath,
      root,
      logParts,
      rasterNotStarted: true,
    });
  }

  const rasterCmd = `${raster} ${script}`;
  logParts.push(`\n# Raster run: ${script}\n$ ${rasterCmd}`);
  console.log(`[compat-${label}] Raster run: ${rasterCmd}`);

  const rasterResult = await spawnCollect(
    raster,
    [script],
    { cwd: directory, env },
    maxDurationMs
  );

  validateCompatRun(label, "Raster run", rasterResult, {
    maxDurationMs,
    expectCode,
    successMarker,
    mustNotContainStdout,
    logPath,
    root,
    logParts,
    rasterNotStarted: false,
  });
}

function validateCompatRun(
  label,
  phase,
  result,
  {
    maxDurationMs,
    expectCode,
    successMarker,
    mustNotContainStdout,
    logPath,
    root,
    logParts,
    rasterNotStarted,
  }
) {
  process.stdout.write(result.stdout);
  process.stderr.write(result.stderr);

  const exitLabel = result.timedOut
    ? `timeout after ${maxDurationMs}ms`
    : String(result.code ?? result.signal);
  logParts.push(
    `exit: ${exitLabel}\n\nstdout:\n${result.stdout}\n\nstderr:\n${result.stderr}`
  );

  if (result.timedOut) {
    throw new Error(
      `${label} ${phase} timed out after ${maxDurationMs}ms. ` +
        `See ${path.relative(root, logPath)}.` +
        (rasterNotStarted ? " Raster was not started." : "")
    );
  }

  if (result.code !== expectCode) {
    throw new Error(
      `${label} ${phase} expected exit ${expectCode}, got ${result.code ?? result.signal}. ` +
        `See ${path.relative(root, logPath)}.` +
        (rasterNotStarted ? " Raster was not started." : "")
    );
  }

  if (successMarker && !result.stdout.includes(successMarker)) {
    throw new Error(
      `${label} ${phase} exited ${expectCode} but stdout missing "${successMarker}". ` +
        `See ${path.relative(root, logPath)}.` +
        (rasterNotStarted ? " Raster was not started." : "")
    );
  }

  if (mustNotContainStdout && result.stdout.includes(mustNotContainStdout)) {
    throw new Error(
      `${label} ${phase} stdout must not contain "${mustNotContainStdout}". ` +
        `See ${path.relative(root, logPath)}.`
    );
  }
}

async function runMysql2ScriptCompat(
  testCase,
  directory,
  raster,
  logPath,
  root
) {
  const logParts = [];
  let containerId = null;
  let cleanupDone = false;
  let signalExitCode = null;

  const appendContainerDiagnostics = async () => {
    if (!containerId) {
      return;
    }
    const [stateStatus, exitCode, health, ports] = await Promise.all([
      execDocker(["inspect", "--format={{.State.Status}}", containerId]),
      execDocker(["inspect", "--format={{.State.ExitCode}}", containerId]),
      execDocker(["inspect", "--format={{.State.Health.Status}}", containerId]),
      execDocker(["port", containerId, "3306/tcp"]),
    ]);
    logParts.push(
      "\n# container diagnostics\n" +
        `state: ${redactSecrets(stateStatus.stdout.trim())}\n` +
        `exit-code: ${redactSecrets(exitCode.stdout.trim())}\n` +
        `health: ${redactSecrets(health.stdout.trim())}\n` +
        `port: ${redactSecrets(ports.stdout.trim())}`
    );
    const logs = await execDocker(["logs", containerId]);
    logParts.push(
      `\n# docker logs\nstdout:\n${redactSecrets(logs.stdout)}\nstderr:\n${redactSecrets(logs.stderr)}`
    );
  };

  const stopContainer = async (reason) => {
    if (!containerId) {
      return { ok: true, message: "no container to stop" };
    }
    const stopResult = await execDocker(["stop", "--time", "5", containerId]);
    if (stopResult.code !== 0) {
      return {
        ok: false,
        message:
          `docker stop failed (exit ${stopResult.code}, ${reason}): ` +
          `${redactSecrets(stopResult.stderr.trim())}`,
      };
    }
    return { ok: true, message: `stopped (${reason})` };
  };

  const cleanup = async (reason) => {
    if (cleanupDone) {
      return { ok: true, message: "already cleaned up" };
    }
    cleanupDone = true;
    let stopResult;
    try {
      stopResult = await stopContainer(reason);
    } catch (err) {
      stopResult = {
        ok: false,
        message: `docker stop error: ${err instanceof Error ? err.message : String(err)}`,
      };
    }
    logParts.push(`\n# Container cleanup\n${stopResult.message}`);
    try {
      await writeLog(logPath, logParts);
    } catch {
      // do not override the original test error
    }
    return stopResult;
  };

  const onSignal = (signal) => {
    if (signalExitCode !== null) {
      return;
    }
    signalExitCode = signal === "SIGINT" ? 130 : 143;
    cleanup(signal).finally(() => process.exit(signalExitCode));
  };

  process.on("SIGINT", onSignal);
  process.on("SIGTERM", onSignal);

  let testFailed = false;
  try {
    logParts.push(`# Docker\nimage: ${MYSQL_DOCKER_IMAGE}`);

    const dockerVersion = await execDocker([
      "version",
      "--format",
      "{{.Server.Version}}",
    ]);
    if (dockerVersion.code !== 0 || !dockerVersion.stdout.trim()) {
      throw new Error(
        "Docker daemon is not available. Install Docker and ensure the daemon is running." +
          (dockerVersion.stderr.trim()
            ? `\n${dockerVersion.stderr.trim()}`
            : "")
      );
    }
    logParts.push(`docker-server: ${dockerVersion.stdout.trim()}`);

    const runArgs = [
      "run",
      "--detach",
      "--rm",
      "--env",
      "MYSQL_ROOT_PASSWORD=compat-root",
      "--env",
      "MYSQL_DATABASE=raster_compat",
      "--env",
      "MYSQL_USER=raster",
      "--env",
      "MYSQL_PASSWORD=raster",
      "--publish",
      `${MYSQL_DOCKER_HOST}::3306`,
      "--health-cmd=mysqladmin ping -h 127.0.0.1 -uroot -pcompat-root --silent",
      "--health-interval=2s",
      "--health-timeout=5s",
      "--health-retries=30",
      MYSQL_DOCKER_IMAGE,
    ];
    logParts.push(
      `\n# docker run\n$ docker ${redactSecrets(runArgs.join(" "))}`
    );

    const runResult = await execDocker(runArgs, MYSQL_DOCKER_RUN_TIMEOUT_MS);
    if (runResult.timedOut) {
      logParts.push(
        `exit: timeout after ${MYSQL_DOCKER_RUN_TIMEOUT_MS / 1000}s`
      );
      throw new Error(
        `Timed out starting MySQL container after ${MYSQL_DOCKER_RUN_TIMEOUT_MS / 1000}s ` +
          "(image pull or container start may be slow)"
      );
    }
    if (runResult.code !== 0) {
      logParts.push(
        `exit: ${runResult.code}\nstderr:\n${redactSecrets(runResult.stderr)}`
      );
      throw new Error(
        `Failed to start MySQL container: ${redactSecrets(runResult.stderr.trim()) || "unknown error"}`
      );
    }

    containerId = runResult.stdout.trim();
    if (!containerId) {
      throw new Error("docker run produced no container ID");
    }
    logParts.push(`container-id: ${containerId}`);

    const portResult = await execDocker(["port", containerId, "3306/tcp"]);
    const port = parseDockerPort(portResult.stdout);
    if (!port) {
      await appendContainerDiagnostics();
      throw new Error(
        `Failed to parse mapped port from: ${redactSecrets(portResult.stdout.trim() || portResult.stderr.trim())}`
      );
    }
    logParts.push(`host: ${MYSQL_DOCKER_HOST}\nport: ${port}`);

    await waitForMysqlHealthy(
      containerId,
      logParts,
      appendContainerDiagnostics
    );

    const testEnv = {
      ...process.env,
      MYSQL_HOST: MYSQL_DOCKER_HOST,
      MYSQL_PORT: port,
      MYSQL_DATABASE: "raster_compat",
      MYSQL_USER: "raster",
      MYSQL_PASSWORD: "raster",
    };
    logParts.push(
      `\n# Database config\nhost: ${MYSQL_DOCKER_HOST}\nport: ${port}\ndatabase: raster_compat\nuser: raster`
    );

    const scripts = testCase.scripts ?? [
      {
        script: testCase.script,
        successMarker: testCase.successMarker,
        maxDurationMs: SCRIPT_TIMEOUT_MS,
        expectCode: 0,
      },
    ];

    for (const spec of scripts) {
      await runCompatScript(
        spec,
        directory,
        raster,
        logParts,
        root,
        logPath,
        testEnv
      );
    }

    const compatMode =
      process.env.COMPAT_SKIP_NODE_BASELINE === "1"
        ? "Raster only"
        : "Node baseline + Raster";
    console.log(
      `${name} compatibility passed (${scripts.length} script(s), ${compatMode})`
    );
  } catch (err) {
    testFailed = true;
    try {
      await appendContainerDiagnostics();
    } catch {
      // ignore diagnostic failures
    }
    throw err;
  } finally {
    process.off("SIGINT", onSignal);
    process.off("SIGTERM", onSignal);
    if (signalExitCode === null) {
      const cleanupResult = await cleanup(testFailed ? "error" : "complete");
      if (!testFailed && !cleanupResult.ok) {
        throw new Error(
          `MySQL container cleanup failed: ${cleanupResult.message}`
        );
      }
    }
  }
}

function redactSecrets(text) {
  if (!text) {
    return "";
  }
  return String(text)
    .replace(/([A-Za-z_]*PASSWORD=)[^\s"']*/gi, "$1***")
    .replace(/(^|\s)-p\S+/gm, "$1-p***");
}

function execDocker(args, timeoutMs = 60_000) {
  return spawnCollect("docker", args, { env: { ...process.env } }, timeoutMs);
}

function parseDockerPort(output) {
  const line = output.trim().split("\n")[0]?.trim();
  if (!line) {
    return null;
  }
  const match = /^127\.0\.0\.1:(\d+)$/.exec(line);
  return match ? match[1] : null;
}

async function waitForMysqlHealthy(containerId, logParts, appendDiagnostics) {
  const deadline = Date.now() + MYSQL_HEALTH_TIMEOUT_MS;
  let lastStatus = null;

  while (Date.now() < deadline) {
    const stateResult = await execDocker([
      "inspect",
      "--format={{.State.Status}}",
      containerId,
    ]);
    if (stateResult.code !== 0) {
      await appendDiagnostics();
      throw new Error("MySQL container no longer exists");
    }

    const stateStatus = stateResult.stdout.trim();
    if (stateStatus === "exited") {
      await appendDiagnostics();
      throw new Error("MySQL container exited before becoming healthy");
    }

    const healthResult = await execDocker([
      "inspect",
      "--format={{.State.Health.Status}}",
      containerId,
    ]);
    const status = healthResult.stdout.trim();
    if (status && status !== lastStatus) {
      logParts.push(`health: ${status}`);
      lastStatus = status;
    }

    if (status === "healthy") {
      return;
    }
    if (status === "unhealthy") {
      await appendDiagnostics();
      throw new Error("MySQL container became unhealthy");
    }

    await sleep(1_000);
  }

  await appendDiagnostics();
  throw new Error(
    `MySQL container health check timed out after ${MYSQL_HEALTH_TIMEOUT_MS / 1000}s`
  );
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

    // Concurrent AsyncLocalStorage isolation across overlapping requests.
    if (!serverExit) {
      const alsIds = ["a1", "b2", "c3", "d4", "e5", "f6"];
      try {
        const results = await Promise.all(
          alsIds.map(async (id) => {
            const signal = AbortSignal.timeout(HTTP_CHECK_TIMEOUT_MS);
            const res = await fetch(`${baseUrl}/api/als/${id}`, { signal });
            const body = await res.text();
            let json;
            try {
              json = JSON.parse(body);
            } catch {
              return {
                id,
                ok: false,
                detail: `invalid JSON status=${res.status} body=${truncate(body, 200)}`,
              };
            }
            const ok = res.status === 200 && json?.id === id;
            return {
              id,
              ok,
              detail: ok
                ? null
                : `status=${res.status} body=${JSON.stringify(json)}`,
            };
          })
        );

        const alsLog = results
          .map((r) => `  ${r.id}: ${r.ok ? "ok" : r.detail}`)
          .join("\n");
        logParts.push(`\n# HTTP concurrent ALS\n${alsLog}`);

        for (const r of results) {
          if (!r.ok) {
            failures.push(`GET /api/als/${r.id}: ${r.detail}`);
          }
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        logParts.push(`\n# HTTP concurrent ALS\nerror: ${msg}`);
        failures.push(`concurrent ALS: request failed: ${msg}`);
      }
    } else {
      failures.push(
        `concurrent ALS: skipped (Raster already exited: ${formatExit(serverExit)})`
      );
      logParts.push(
        `\n# HTTP concurrent ALS\nskipped: Raster exited ${formatExit(serverExit)}`
      );
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
        "(Node build + Raster run; HTTP / /api/health /posts/42 + concurrent ALS OK)"
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
 * Spawn a process and verify it is still running at `checkMs`, then kill it.
 */
function spawnStillRunning(command, args, options, checkMs, killAfterMs) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      ...options,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    let exited = false;
    let exitCode = null;

    child.stdout.on("data", (chunk) => (stdout += chunk));
    child.stderr.on("data", (chunk) => (stderr += chunk));
    child.on("error", reject);
    child.on("close", (code) => {
      exited = true;
      exitCode = code;
    });

    setTimeout(() => {
      if (exited) {
        reject(
          new Error(
            `process exited before ${checkMs}ms (exit ${exitCode ?? "signal"})`
          )
        );
        return;
      }
      const killDelay = Math.max(0, killAfterMs - checkMs);
      setTimeout(() => {
        try {
          child.kill("SIGTERM");
        } catch {
          // ignore
        }
        setTimeout(() => {
          resolve({ stdout, stderr, stillAliveAtCheck: true });
        }, 100);
      }, killDelay);
    }, checkMs);
  });
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
