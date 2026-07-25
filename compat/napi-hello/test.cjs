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

console.log("napi-hello compat OK");
