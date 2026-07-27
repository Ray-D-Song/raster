"use strict";

const path = require("node:path");

const addonPath = path.join(__dirname, "build", "Release", "hello.node");
const hello = require(addonPath);

// Start the delayed unref TSFN first, then keep the event loop alive briefly.
hello.delayedTsfnUnrefExit((value) => {
  if (value !== 88) {
    process.exit(1);
  }
  console.log("timer-tsfn-ok");
  process.exit(0);
});

setTimeout(() => {
  process.exit(2);
}, 200);

setTimeout(() => {
  process.exit(3);
}, 2000);
