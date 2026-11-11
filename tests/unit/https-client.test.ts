import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnCapture } from "./test-utils";

// `https.get`/`https.request` are the only client entry points that need TLS at
// all, and every case below failed on main:
//   - the URI defaulted to `http:`, so a TLS port got a plaintext request;
//   - per-request `ca`/`rejectUnauthorized` were dropped, because the shared
//     pooled client is built once from the process-wide TLS config;
//   - transport failures were thrown instead of emitted as `error`.
//
// Each case runs entirely inside a child process: it starts its own listener,
// drives the client, prints one line, and exits. Nothing here creates a socket
// or timer in the test worker, because a listener in this worker cannot be torn
// down cleanly and would fail the run for reasons unrelated to the assertions.

const DATA_DIR = path.resolve("libs/raster_runtime_test_tls/data");
const { RASTER_RUNTIME_LOG, ...TEST_ENV } = process.env;

let scriptDir = "";

beforeAll(() => {
  scriptDir = fs.mkdtempSync(path.join(os.tmpdir(), "raster-https-client-"));
});

afterAll(() => {
  fs.rmSync(scriptDir, { recursive: true, force: true });
});

let scriptSeq = 0;

/// Run a self-contained client script in a child process; return its stdout.
const runClient = async (source: string): Promise<string> => {
  const file = path.join(scriptDir, `client-${scriptSeq++}.mjs`);
  fs.writeFileSync(file, source);
  const { stdout, stderr } = await spawnCapture(process.argv[0], [file], {
    env: { ...TEST_ENV } as Record<string, string>,
  });
  const out = stdout.trim();
  if (!out) throw new Error(`client produced no output; stderr: ${stderr}`);
  return out;
};

/// Preamble shared by the TLS cases: a listener that answers one request with a
/// fixed HTTP/1.1 response, echoing any request body back as `got:<body>`.
const tlsServerPreamble = `
import tls from "node:tls";
import fs from "node:fs";
const DATA = ${JSON.stringify(DATA_DIR)};
const server = tls.createServer(
  {
    key: fs.readFileSync(DATA + "/server.key"),
    cert: fs.readFileSync(DATA + "/server.pem"),
  },
  (socket) => {
    socket.on("error", () => {});
    let raw = "";
    const onData = (chunk) => {
      raw += chunk.toString();
      const headerEnd = raw.indexOf("\\r\\n\\r\\n");
      if (headerEnd === -1) return;
      const head = raw.slice(0, headerEnd).toLowerCase();
      const rest = raw.slice(headerEnd + 4);
      const method = head.split(" ")[0].toUpperCase();
      const declared = head.includes("content-length:")
        ? Number(head.split("content-length:")[1].split("\\r\\n")[0])
        : 0;
      const finish = (body) => {
        socket.removeListener("data", onData);
        const payload = "got:" + body;
        socket.end(
          [
            "HTTP/1.1 200 OK",
            "content-type: text/plain",
            "x-method: " + method,
            "content-length: " + Buffer.byteLength(payload),
            "connection: close",
            "",
            payload,
          ].join("\\r\\n"),
          () => socket.destroy()
        );
      };
      if (declared === 0) return finish(rest);
      if (rest.length >= declared) return finish(rest.slice(0, declared));
      socket.on("data", (more) => {
        raw += more.toString();
        const body = raw.slice(headerEnd + 4);
        if (body.length >= declared) finish(body.slice(0, declared));
      });
    };
    socket.on("data", onData);
  }
);
const port = await new Promise((resolve) => {
  server.listen(() => resolve(server.address().port));
});
const done = (line) => {
  console.log(line);
  server.close();
  process.exit(0);
};
`;

it("https.get defaults to https: and reads the response body", async () => {
  const out = await runClient(`
${tlsServerPreamble}
import https from "node:https";
https.get({ port, host: "127.0.0.1", rejectUnauthorized: false, path: "/" }, (res) => {
  let body = "";
  res.on("data", (c) => (body += c));
  res.on("end", () => done(res.statusCode + "|" + body));
}).on("error", (e) => done("ERR|" + e.message));
`);
  expect(out).toBe("200|got:");
}, 30000);

it("https.request sends a request body and reads the response body", async () => {
  const out = await runClient(`
${tlsServerPreamble}
import https from "node:https";
const body = "payload";
const req = https.request(
  {
    port,
    host: "127.0.0.1",
    rejectUnauthorized: false,
    path: "/",
    method: "POST",
    headers: { "content-length": body.length },
  },
  (res) => {
    let got = "";
    res.on("data", (c) => (got += c));
    res.on("end", () =>
      done(res.statusCode + "|" + res.headers["x-method"] + "|" + got)
    );
  }
);
req.on("error", (e) => done("ERR|" + e.message));
req.write(body);
req.end();
`);
  expect(out).toBe("200|POST|got:payload");
}, 30000);

it("https.get honours a per-request `ca`", async () => {
  const out = await runClient(`
${tlsServerPreamble}
import https from "node:https";
https.get({ port, host: "127.0.0.1", ca: fs.readFileSync(DATA + "/root.pem"), path: "/" }, (res) => {
  let body = "";
  res.on("data", (c) => (body += c));
  res.on("end", () => done(res.statusCode + "|" + body));
}).on("error", (e) => done("ERR|" + e.message));
`);
  expect(out).toBe("200|got:");
}, 30000);

it("https.get reports verification failures through the error event", async () => {
  // No `ca` and no `rejectUnauthorized: false`: the self-signed chain must be
  // rejected, and the failure must arrive as an `error` event rather than as an
  // uncaught throw (which is how it used to escape).
  const out = await runClient(`
${tlsServerPreamble}
import https from "node:https";
const req = https.get({ port, host: "127.0.0.1", path: "/" });
req.on("response", () => done("UNEXPECTED_RESPONSE"));
req.on("error", (e) => done("ERROR_EVENT|" + typeof e.message));
`);
  expect(out).toBe("ERROR_EVENT|string");
}, 30000);

it("plain http.get is unaffected", async () => {
  const out = await runClient(`
import http from "node:http";
const server = http.createServer((_req, res) => {
  res.writeHead(200, { "content-type": "text/plain" });
  res.end("plain");
});
const port = await new Promise((resolve) => {
  server.listen(() => resolve(server.address().port));
});
http.get({ port, host: "127.0.0.1", path: "/" }, (res) => {
  let body = "";
  res.on("data", (c) => (body += c));
  res.on("end", () => {
    console.log(res.statusCode + "|" + body);
    server.close();
    process.exit(0);
  });
}).on("error", (e) => {
  console.log("ERR|" + e.message);
  process.exit(0);
});
`);
  expect(out).toBe("200|plain");
}, 30000);
