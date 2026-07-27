"use strict";

const path = require("node:path");

const addonPath = path.join(__dirname, "build", "Release", "hello.node");
const hello = require(addonPath);

function assert(condition, message) {
  if (!condition) {
    console.error(message);
    process.exit(1);
  }
}

const result = hello.hello();
assert(result === "world", `hello() expected "world", got ${JSON.stringify(result)}`);

assert(hello.removeWrapTest() === 0, "removeWrapTest expected finalize count 0");

function waitAsync(label, fn) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      reject(new Error(`${label} timed out`));
    }, 2000);
    fn((value) => {
      clearTimeout(timer);
      resolve(value);
    });
  });
}

(async () => {
  const asyncResult = await waitAsync("queueAsyncWork", (done) => {
    hello.queueAsyncWork(done);
  });
  assert(asyncResult === 42, `queueAsyncWork expected 42, got ${asyncResult}`);

  const tsfnResult = await waitAsync("callTsfnFromThread", (done) => {
    hello.callTsfnFromThread(done);
  });
  assert(tsfnResult === 99, `callTsfnFromThread expected 99, got ${tsfnResult}`);

  hello.createStoredTsfn(() => {});
  hello.unrefStoredTsfn();
  hello.releaseStoredTsfn();

  const delayedResult = await waitAsync("delayedTsfnUnrefExit", (done) => {
    hello.delayedTsfnUnrefExit(done);
  });
  assert(delayedResult === 88, `delayedTsfnUnrefExit expected 88, got ${delayedResult}`);

  const requireAwaitOk = await waitAsync("tsfnRequireAwaitModule", (done) => {
    hello.tsfnRequireAwaitModule(
      (ok) => {
        done(ok);
      },
      path.join(__dirname, "await-mod.mjs"),
    );
  });
  assert(requireAwaitOk === true, `tsfnRequireAwaitModule expected true, got ${requireAwaitOk}`);

  hello.createStoredTsfn(() => {});
  await new Promise((resolve) => setTimeout(resolve, 50));
  hello.unrefStoredTsfn();

  console.log("napi-hello compat OK");
})().catch((err) => {
  console.error(err);
  process.exit(1);
});
