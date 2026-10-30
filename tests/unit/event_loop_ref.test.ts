import { spawnCapture } from "./test-utils";

describe("event loop ref/unref", () => {
  it("unref timer allows process to exit before callback", async () => {
    const { code, stdout } = await spawnCapture(process.argv[0], [
      "-e",
      `
        setTimeout(() => console.log("late"), 500).unref();
        console.log("done");
      `,
    ]);
    expect(code).toBe(0);
    expect(stdout.trim()).toBe("done");
  });

  it("ref timer keeps process alive until callback", async () => {
    const { code, stdout } = await spawnCapture(process.argv[0], [
      "-e",
      `
        setTimeout(() => console.log("fired"), 30);
      `,
    ]);
    expect(code).toBe(0);
    expect(stdout.trim()).toBe("fired");
  });

  it("timer ref() after unref() keeps process alive", async () => {
    const { code, stdout } = await spawnCapture(process.argv[0], [
      "-e",
      `
        const t = setTimeout(() => console.log("ok"), 30);
        t.unref();
        t.ref();
      `,
    ]);
    expect(code).toBe(0);
    expect(stdout.trim()).toBe("ok");
  });

  it("timers/promises { ref: false } does not block exit alone", async () => {
    const { code, stdout } = await spawnCapture(process.argv[0], [
      "-e",
      `
        import("node:timers/promises").then(({ setTimeout }) => {
          setTimeout(500, "x", { ref: false });
          console.log("main");
        });
      `,
    ]);
    expect(code).toBe(0);
    expect(stdout.trim()).toBe("main");
  });

  it("unref net server allows exit while listening", async () => {
    const { code, stdout } = await spawnCapture(process.argv[0], [
      "-e",
      `
        const net = require("net");
        const s = net.createServer();
        s.listen(0, () => {
          s.unref();
          console.log("done");
        });
      `,
    ]);
    expect(code).toBe(0);
    expect(stdout.trim()).toBe("done");
  });

  it("unref dgram socket allows exit while bound", async () => {
    const { code, stdout } = await spawnCapture(process.argv[0], [
      "-e",
      `
        const dgram = require("dgram");
        const s = dgram.createSocket("udp4");
        s.bind(0, () => {
          s.unref();
          console.log("done");
        });
      `,
    ]);
    expect(code).toBe(0);
    expect(stdout.trim()).toBe("done");
  });
});
