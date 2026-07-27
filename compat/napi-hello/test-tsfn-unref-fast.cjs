"use strict";

const path = require("node:path");

const addonPath = path.join(__dirname, "build", "Release", "hello.node");
const hello = require(addonPath);

// Unref'd TSFN with no timers: process must exit before the delayed callback (88).
hello.delayedTsfnUnrefExit(() => {
  process.exit(2);
});
