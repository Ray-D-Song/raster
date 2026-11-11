import http from "node:http";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import { startFixtureSource } from "./ws-test-helpers";

// `http.WebSocket` is the Node 22+ WebSocket client. Every case runs entirely in
// a child process — it starts the Python echo fixture from `fixtures/`, drives
// the client, prints one line and exits.
//
// The test worker must never own a listener or a long-lived child: either one
// left behind makes the worker exit non-zero at teardown, which fails the run
// for reasons unrelated to the assertion. `https-client.test.ts` follows the
// same shape.

let scriptDir = "";

beforeAll(() => {
  scriptDir = fs.mkdtempSync(path.join(os.tmpdir(), "raster-ws-"));
});

afterAll(() => {
  if (scriptDir) fs.rmSync(scriptDir, { recursive: true, force: true });
});

let scriptSeq = 0;

/// Run a client script in a child process and return the line it prints.
const runClient = (source: string): Promise<string> =>
  new Promise((resolve, reject) => {
    const file = path.join(scriptDir, `client-${scriptSeq++}.mjs`);
    fs.writeFileSync(file, source);
    const child = spawn(process.argv[0], [file], { stdio: ["ignore", "pipe", "pipe"] });
    let out = "";
    let err = "";
    child.stdout.on("data", (chunk: any) => (out += chunk.toString()));
    child.stderr.on("data", (chunk: any) => (err += chunk.toString()));
    child.on("close", () => {
      const line = out.trim();
      if (line) resolve(line);
      else reject(new Error(`client produced no output; stderr: ${err}`));
    });
    child.on("error", reject);
  });

it("exposes the WebSocket class and readyState constants", async () => {
  const out = await runClient(`
${startFixtureSource("echo")}
import http from "node:http";
const ws = new http.WebSocket("ws://127.0.0.1:" + fixturePort + "/");
console.log(
  typeof http.WebSocket === "function" ? "class" : "missing",
  ws.CONNECTING, ws.OPEN, ws.CLOSING, ws.CLOSED,
  ws.readyState, JSON.stringify(ws.binaryType), ws.url
);
ws.onopen = () => { stopFixture(); process.exit(0); };
ws.onerror = () => { stopFixture(); process.exit(0); };
setTimeout(() => { stopFixture(); process.exit(0); }, 3000);
`);
  // `url` carries the fixture's ephemeral port, so only the prefix is fixed.
  expect(out.startsWith('class 0 1 2 3 0 "blob" ws://127.0.0.1:')).toBe(true);
}, 20000);

it("round-trips text through open/message/close", async () => {
  const out = await runClient(`
${startFixtureSource("echo")}
import http from "node:http";
const ws = new http.WebSocket("ws://127.0.0.1:" + fixturePort + "/");
const seen = [];
ws.onopen = () => { seen.push("open:" + ws.readyState); ws.send("hello"); };
ws.onmessage = (ev) => { seen.push("message:" + ev.data); ws.close(1000, "done"); };
ws.onclose = (ev) => {
  seen.push("close:" + ev.code + ":" + ev.reason + ":" + ev.wasClean + ":" + ws.readyState);
  console.log(seen.join("|"));
  stopFixture();
  process.exit(0);
};
ws.onerror = (ev) => { console.log("ERROR:" + (ev && ev.message)); stopFixture(); process.exit(1); };
setTimeout(() => { console.log("TIMEOUT:" + seen.join(",")); stopFixture(); process.exit(1); }, 6000);
`);
  expect(out).toBe("open:1|message:hello|close:1000:done:true:3");
}, 20000);

it("round-trips binary as Buffer and as ArrayBuffer", async () => {
  const out = await runClient(`
${startFixtureSource("echo")}
import http from "node:http";
const ws = new http.WebSocket("ws://127.0.0.1:" + fixturePort + "/");
const seen = [];
ws.onopen = () => ws.send(Buffer.from([1, 2, 3, 4, 5]));
ws.onmessage = (ev) => {
  if (seen.length === 0) {
    seen.push("buffer:" + (Buffer.isBuffer(ev.data) ? ev.data.toString("hex") : "not-buffer"));
    ws.binaryType = "arraybuffer";
    ws.send(new Uint8Array([9, 8, 7]).buffer);
    return;
  }
  seen.push("arraybuffer:" + (ev.data instanceof ArrayBuffer
    ? Array.from(new Uint8Array(ev.data)).join(",")
    : "not-arraybuffer"));
  ws.close();
};
ws.onclose = () => { console.log(seen.join("|")); stopFixture(); process.exit(0); };
ws.onerror = (ev) => { console.log("ERROR:" + (ev && ev.message)); stopFixture(); process.exit(1); };
setTimeout(() => { console.log("TIMEOUT:" + seen.join(",")); stopFixture(); process.exit(1); }, 6000);
`);
  expect(out).toBe("buffer:0102030405|arraybuffer:9,8,7");
}, 20000);

it("round-trips a message larger than 64KB", async () => {
  const out = await runClient(`
${startFixtureSource("echo")}
import http from "node:http";
const ws = new http.WebSocket("ws://127.0.0.1:" + fixturePort + "/");
const payload = "x".repeat(200 * 1024);
ws.onopen = () => ws.send(payload);
ws.onmessage = (ev) => {
  const ok = typeof ev.data === "string" && ev.data.length === payload.length && ev.data === payload;
  console.log("len:" + (ev.data && ev.data.length) + ":match:" + ok);
  stopFixture();
  process.exit(0);
};
ws.onerror = (ev) => { console.log("ERROR:" + (ev && ev.message)); stopFixture(); process.exit(1); };
setTimeout(() => { console.log("TIMEOUT"); stopFixture(); process.exit(1); }, 15000);
`);
  expect(out).toBe("len:204800:match:true");
}, 30000);

it("reassembles fragmented messages and answers ping", async () => {
  // The `fragmented` fixture sends "he" + "ll" + "o" as three frames plus a ping.
  const out = await runClient(`
${startFixtureSource("fragmented")}
import http from "node:http";
const ws = new http.WebSocket("ws://127.0.0.1:" + fixturePort + "/");
let messages = 0;
ws.onmessage = (ev) => {
  messages++;
  console.log("message:" + JSON.stringify(ev.data) + ":" + messages);
  stopFixture();
  process.exit(0);
};
ws.onclose = () => { console.log("CLOSED_BEFORE_MESSAGE"); stopFixture(); process.exit(1); };
ws.onerror = (ev) => { console.log("ERROR:" + (ev && ev.message)); stopFixture(); process.exit(1); };
setTimeout(() => { console.log("TIMEOUT"); stopFixture(); process.exit(1); }, 6000);
`);
  // One message, not three: continuations belong to a single logical message.
  expect(out).toBe('message:"hello":1');
}, 20000);

it("reports a bad Sec-WebSocket-Accept instead of accepting it", async () => {
  const out = await runClient(`
import http from "node:http";
import net from "node:net";
const server = net.createServer((sock) => {
  let raw = "";
  sock.on("data", (d) => {
    raw += d.toString();
    if (!raw.includes("\\r\\n\\r\\n")) return;
    // 101 with the right headers but a token that cannot match our key.
    sock.write(
      "HTTP/1.1 101 Switching Protocols\\r\\n" +
      "Upgrade: websocket\\r\\n" +
      "Connection: Upgrade\\r\\n" +
      "Sec-WebSocket-Accept: AAAAAAAAAAAAAAAAAAAAAAAAAAA=\\r\\n\\r\\n"
    );
  });
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const ws = new http.WebSocket("ws://127.0.0.1:" + server.address().port + "/");
ws.onopen = () => { console.log("UNEXPECTED_OPEN"); process.exit(1); };
ws.onerror = (ev) => {
  console.log("error:" + (ev && ev.message));
};
ws.onclose = (ev) => {
  console.log("close:" + ev.code);
  server.close();
  process.exit(0);
};
setTimeout(() => { console.log("TIMEOUT"); process.exit(1); }, 6000);
`);
  expect(out).toContain("invalid Sec-WebSocket-Accept");
  expect(out).toContain("close:1006");
}, 20000);

it("reports a refused connection", async () => {
  const out = await runClient(`
import http from "node:http";
const ws = new http.WebSocket("ws://127.0.0.1:1/");
ws.onopen = () => { console.log("UNEXPECTED_OPEN"); process.exit(1); };
ws.onerror = (ev) => console.log("error:" + (ev && ev.message ? "has-message" : "no-message"));
ws.onclose = (ev) => { console.log("close:" + ev.code + ":" + ws.readyState); process.exit(0); };
setTimeout(() => { console.log("TIMEOUT"); process.exit(1); }, 6000);
`);
  expect(out).toContain("error:has-message");
  expect(out).toContain("close:1006:3");
}, 20000);

it("rejects a non-websocket scheme", async () => {
  const out = await runClient(`
import http from "node:http";
let threw = false;
try {
  new http.WebSocket("http://127.0.0.1:1/");
} catch (err) {
  threw = String(err.message).includes("ws:");
}
console.log("threw:" + threw);
`);
  expect(out).toBe("threw:true");
}, 20000);
