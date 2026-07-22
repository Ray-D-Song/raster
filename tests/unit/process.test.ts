import process from "node:process";

import defaultImport from "node:process";
import legacyImport from "process";
import { spawnCapture } from "./test-utils";

it("node:process should be the same as process", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const {
  env,
  cwd,
  argv0,
  argv,
  platform,
  arch,
  hrtime,
  release,
  version,
  versions,
  exit,
  on,
  once,
  off,
  emit,
} = defaultImport;

it("should have a process env", () => {
  expect(env).toEqual(process.env);
});

it("should have a process cwd", () => {
  expect(cwd()).toEqual(process.cwd());
});

it("should have a process argv0", () => {
  expect(argv0).toEqual(process.argv0);
});

it("should have a process argv", () => {
  expect(argv).toEqual(process.argv);
});

it("should have a process platform", () => {
  expect(platform).toEqual(process.platform);
});

it("should have a process arch", () => {
  expect(arch).toEqual(process.arch);
});

it("should have a process hrtime", () => {
  expect(hrtime.bigint() > 0).toBeTruthy();
});

it("should have a process release", () => {
  expect(release).toEqual(process.release);
});

it("should have a process version", () => {
  expect(version).toEqual(process.version);
});

it("should have a process versions", () => {
  expect(versions).toEqual(process.versions);
});

it("should have a process exit", () => {
  expect(exit).toEqual(process.exit);
});

describe("Node compat identity", () => {
  it("should advertise Node 22.18.0 identity", () => {
    expect(process.version).toBe("v22.18.0");
    expect(process.versions.node).toBe("22.18.0");
    expect(typeof process.versions.raster_runtime).toBe("string");
    expect(process.versions.raster_runtime.length > 0).toBeTruthy();
    expect(process.release.name).toBe("raster_runtime");
  });
});

describe("EventEmitter methods", () => {
  it("should register, emit, and remove listeners with on/once/off/emit", () => {
    const seen: number[] = [];
    const listener = (n: number) => {
      seen.push(n);
    };

    process.on("compat-test", listener);
    process.emit("compat-test", 1);
    process.off("compat-test", listener);
    process.emit("compat-test", 2);
    expect(seen).toEqual([1]);
  });

  it("once should only fire once", () => {
    let count = 0;
    process.once("compat-once", () => {
      count += 1;
    });
    process.emit("compat-once");
    process.emit("compat-once");
    expect(count).toBe(1);
  });

  it("process.exit should emit exit synchronously before terminating", async () => {
    const { code, stderr } = await spawnCapture(process.argv[0], [
      "-e",
      `
        process.on("exit", (code) => {
          console.error("exit:" + code);
        });
        process.exit(42);
      `,
    ]);
    expect(code).toBe(42);
    expect(stderr).toContain("exit:42");
  });

  it("should not expose internal test helpers on process", () => {
    expect((process as any).__emitExit).toBeUndefined();
    expect(Object.keys(process).includes("__emitExit")).toBeFalsy();
  });

  it("EventEmitter methods should not be own enumerable properties", () => {
    expect(Object.prototype.hasOwnProperty.call(process, "on")).toBeFalsy();
    expect(Object.keys(process).includes("on")).toBeFalsy();
    expect(typeof process.on).toBe("function");
  });

  it("dynamic import should ignore user-added enumerable process fields", async () => {
    (process as any).userField = 123;
    try {
      const mod = await import("process");
      expect(mod.default).toBe(process);
      expect((mod as any).userField).toBeUndefined();
      expect(mod.on).toBe(process.on);
    } finally {
      delete (process as any).userField;
    }
  });

  it("global process and node:process should share event methods", () => {
    expect(process.on).toBe(on);
    expect(process.once).toBe(once);
    expect(process.off).toBe(off);
    expect(process.emit).toBe(emit);
    expect(defaultImport.on).toBe(process.on);
    expect(defaultImport).toBe(process);
    expect(globalThis.process).toBe(process);
  });
});

describe("unhandledRejection timing", () => {
  it("should not emit unhandledRejection when catch attaches in the same turn", async () => {
    let seen = 0;
    const listener = () => {
      seen += 1;
    };
    process.on("unhandledRejection", listener);
    try {
      const p = Promise.reject(new Error("handled"));
      p.catch(() => {});
      await new Promise((r) => setImmediate(r));
      expect(seen).toBe(0);
    } finally {
      process.off("unhandledRejection", listener);
    }
  });

  it("should emit unhandledRejection for truly unhandled rejections", async () => {
    let seen = 0;
    const listener = () => {
      seen += 1;
    };
    process.on("unhandledRejection", listener);
    try {
      Promise.reject(new Error("unhandled"));
      await new Promise((r) => setImmediate(r));
      expect(seen).toBe(1);
    } finally {
      process.off("unhandledRejection", listener);
    }
  });

  it("should not leak rejection tracker helpers on globalThis", () => {
    expect((globalThis as any).__rasterRejectionPending).toBeUndefined();
    expect((globalThis as any).__rasterRejectionSchedule).toBeUndefined();
  });
});
