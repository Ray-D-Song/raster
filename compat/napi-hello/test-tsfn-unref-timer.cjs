"use strict";

const path = require("node:path");

const addonPath = path.join(__dirname, "build", "Release", "hello.node");
const hello = require(addonPath);

// Keep the event loop alive briefly; success path exits via normal teardown.
const timeout = setTimeout(() => {
  console.error("delayed TSFN callback did not run");
  process.exitCode = 2;
}, 200);

hello.delayedTsfnUnrefExit((value) => {
  clearTimeout(timeout);

  if (value !== 88) {
    process.exitCode = 1;
    return;
  }

  console.log("timer-tsfn-ok");
});
