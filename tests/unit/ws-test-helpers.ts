// Helpers for driving `http.WebSocket` from a child process.
//
// Every case in `http-websocket.test.ts` runs entirely inside a child: it starts
// the Python echo fixture, drives the client, prints one line and exits. The
// test worker never owns a socket or a long-lived child — a listener or a
// detached process left in the worker makes it exit non-zero at teardown, which
// would fail the run for reasons unrelated to what is being tested.

/// Source for starting the fixture inside a child script.
///
/// `MODE` is substituted by the caller. The helper resolves the fixture path
/// relative to the repository root, which the child inherits as its cwd.
export const startFixtureSource = (mode: string): string => `
import { spawn } from "node:child_process";
import path from "node:path";

const FIXTURE = path.resolve("tests/unit/fixtures/ws_echo_server.py");
const fixture = spawn("python3", [FIXTURE, "0", ${JSON.stringify(mode)}], {
  stdio: ["ignore", "pipe", "ignore"],
});
let fixturePort = 0;
await new Promise((resolve, reject) => {
  let buffered = "";
  fixture.stdout.on("data", (chunk) => {
    buffered += chunk.toString();
    const match = /^(\\d+)/m.exec(buffered);
    if (match) {
      fixturePort = Number(match[1]);
      resolve();
    }
  });
  fixture.on("error", reject);
  setTimeout(() => reject(new Error("ws fixture did not start")), 5000);
});
const stopFixture = () => { try { fixture.kill(); } catch {} };
`;
