"use strict";
// Must resolve ./stream to local stream.js, not the builtin stream module.
const localStream = require("./stream");
const builtinStream = require("stream");

module.exports = {
  localStream,
  builtinStream,
  localKind: localStream.kind,
  localIsBuiltin: localStream === builtinStream,
  localFilename: module.filename,
};
