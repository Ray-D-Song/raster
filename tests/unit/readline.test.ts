// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
import {
  createInterface,
  clearLine,
  clearScreenDown,
  cursorTo,
  moveCursor,
  emitKeypressEvents,
  promises as readlinePromises,
} from "node:readline";
import { PassThrough } from "node:stream";
import { styleText, stripVTControlCharacters } from "node:util";
import process, {
  stdin,
  stdout,
  stderr,
  version,
  versions,
} from "node:process";
import { isatty, ReadStream, WriteStream } from "node:tty";

describe("readline", () => {
  it("exports Node 24.3 surface", () => {
    expect(typeof createInterface).toBe("function");
    expect(typeof clearLine).toBe("function");
    expect(typeof clearScreenDown).toBe("function");
    expect(typeof cursorTo).toBe("function");
    expect(typeof moveCursor).toBe("function");
    expect(typeof emitKeypressEvents).toBe("function");
    expect(typeof readlinePromises.createInterface).toBe("function");
    expect(typeof readlinePromises.Readline).toBe("function");
  });

  it("parses lines across chunk boundaries", async () => {
    const input = new PassThrough();
    const rl = createInterface({ input, crlfDelay: Infinity });
    const lines: string[] = [];
    rl.on("line", (line: string) => lines.push(line));

    input.write("a\r");
    input.write("\nb\u2028c");
    input.end();

    await new Promise<void>((resolve, reject) => {
      rl.once("close", () => resolve());
      rl.once("error", reject);
    });
    expect(lines).toEqual(["a", "b", "c"]);
  });

  it("supports promises, abort, and async iteration", async () => {
    const input = new PassThrough();
    const output = new PassThrough();
    const rl = readlinePromises.createInterface({ input, output });

    const lines: string[] = [];
    const iterPromise = (async () => {
      for await (const line of rl as any) {
        // Must yield strings, not args arrays from EventEmitter.on
        expect(typeof line).toBe("string");
        expect(Array.isArray(line)).toBe(false);
        lines.push(line);
      }
    })();

    setTimeout(() => {
      input.write("hello\n");
      input.write("world\n");
      input.end();
    }, 0);

    await iterPromise;
    expect(lines).toEqual(["hello", "world"]);

    const input2 = new PassThrough();
    const output2 = new PassThrough();
    const rl2 = readlinePromises.createInterface({
      input: input2,
      output: output2,
    });
    const ac = new AbortController();
    const q = rl2.question("Q? ", { signal: ac.signal });
    ac.abort(new Error("cancelled"));
    let aborted = false;
    try {
      await q;
    } catch (e: any) {
      aborted = true;
      expect(e.name).toBe("AbortError");
    }
    expect(aborted).toBe(true);
    rl2.close();
  });

  it("supports terminal editing via write(key)", () => {
    // Node swaps in a reduced _ttyWrite when TERM=dumb; force a capable TERM
    // so full key editing (left/backspace) is available for this probe.
    const prevTerm = process.env.TERM;
    process.env.TERM = "xterm-256color";
    try {
      const input = new PassThrough();
      const output = new PassThrough();
      const rl = createInterface({
        input,
        output,
        terminal: true,
        historySize: 10,
      });

      (rl as any).write("h");
      (rl as any).write("i");
      expect((rl as any).line).toBe("hi");

      (rl as any).write("", { name: "left" });
      (rl as any).write("X");
      expect((rl as any).line).toBe("hXi");

      (rl as any).write("", { name: "backspace" });
      expect((rl as any).line).toBe("hi");

      rl.close();
    } finally {
      if (prevTerm === undefined) delete process.env.TERM;
      else process.env.TERM = prevTerm;
    }
  });

  it("cursor and clear helpers write ANSI", () => {
    const chunks: string[] = [];
    const stream = {
      write(data: string, cb?: Function) {
        chunks.push(String(data));
        if (cb) cb(null);
        return true;
      },
    };
    cursorTo(stream as any, 2, 3);
    moveCursor(stream as any, 1, -1);
    clearLine(stream as any, 0);
    clearScreenDown(stream as any);
    expect(chunks.length).toBeGreaterThanOrEqual(4);
    expect(chunks.some((c) => c.includes("\x1b["))).toBe(true);
  });

  it("emitKeypressEvents fires keypress for Buffer data", async () => {
    const input = new PassThrough();
    emitKeypressEvents(input as any);
    const got: any[] = [];
    input.on("keypress", (s: string, key: any) => {
      got.push({ s, name: key && key.name });
    });
    input.write(Buffer.from("a"));
    // Allow keypress decoder to process
    await new Promise((r) => setTimeout(r, 0));
    expect(got.length).toBeGreaterThanOrEqual(1);
    expect(got[0].s).toBe("a");
    expect(got[0].name).toBe("a");
    input.end();
  });
});

describe("process stdio identity", () => {
  it("exports Node 24.3 stdio identity", () => {
    expect(version).toBe("v24.3.0");
    expect(versions.node).toBe("24.3.0");
    expect(stdin).toBe((globalThis as any).process.stdin);
    expect(stdout).toBe((globalThis as any).process.stdout);
    expect(stderr).toBe((globalThis as any).process.stderr);
    expect(typeof stdin.on).toBe("function");
    expect(typeof stdout.write).toBe("function");
    expect(typeof isatty).toBe("function");
    expect(typeof ReadStream).toBe("function");
    expect(typeof WriteStream).toBe("function");
  });
});

describe("util.styleText / stripVTControlCharacters", () => {
  it("styles text and strips VT sequences", () => {
    const styled = styleText("red", "hi", { validateStream: false });
    expect(styled).toContain("hi");
    expect(styled).toContain("\x1b[");
    const plain = stripVTControlCharacters(styled);
    expect(plain).toBe("hi");
    expect(styleText("green", "ok", { validateStream: false })).toContain("ok");
  });
});
