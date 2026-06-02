import { spawn } from "node:child_process";
import { mkdirSync, rmSync } from "node:fs";

const running = new Set();
let shuttingDown = false;
const debugLogPath = "./tmp/debug.log";

function runOnce(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: "inherit" });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${command} ${args.join(" ")} exited with ${signal ?? code}`));
      }
    });
  });
}

function start(command, args) {
  const child = spawn(command, args, { stdio: "inherit" });
  running.add(child);

  child.on("exit", (code, signal) => {
    running.delete(child);
    if (shuttingDown) {
      return;
    }
    shutdown(code ?? (signal ? 1 : 0));
  });

  child.on("error", (error) => {
    console.error(error);
    if (!shuttingDown) {
      shutdown(1);
    }
  });

  return child;
}

function shutdown(code) {
  if (shuttingDown) {
    return;
  }
  shuttingDown = true;

  for (const child of running) {
    child.kill("SIGTERM");
  }

  setTimeout(() => {
    for (const child of running) {
      child.kill("SIGKILL");
    }
    process.exit(code);
  }, 1_000).unref();
}

process.on("SIGINT", () => shutdown(130));
process.on("SIGTERM", () => shutdown(143));

try {
  mkdirSync("./tmp", { recursive: true });
  rmSync(debugLogPath, { force: true });

  await runOnce("pnpm", ["run", "build:runtime"]);
  await runOnce("pnpm", ["run", "build:demo"]);

  start("pnpm", ["--dir", "apps/demo", "dev"]);
  start("cargo", ["run", "--release", "--", "--dev", "--log-file", debugLogPath]);
} catch (error) {
  console.error(error);
  shutdown(1);
}
