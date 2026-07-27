"use strict";

const path = require("node:path");

const addonPath = path.join(__dirname, "build", "Release", "hello.node");
const hello = require(addonPath);

// Referenced TSFN with no release and no timers must keep the process alive.
hello.createStoredTsfn(() => {});
