"use strict";

const path = require("node:path");

const addonPath = path.join(__dirname, "build", "Release", "hello.node");
const hello = require(addonPath);

hello.queueAsyncWork((result) => {
  if (result !== 42) {
    process.exit(1);
  }
  console.log("async-work-notimer-ok");
  process.exit(0);
});
