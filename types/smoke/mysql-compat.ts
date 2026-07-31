import net from "node:net";
import crypto from "node:crypto";
import { Buffer } from "buffer";

declare function assert(condition: boolean): void;

const socket = net.connect(3306).setNoDelay().setKeepAlive(true, 0);
assert(typeof socket.setNoDelay === "function");
assert(typeof socket.setKeepAlive === "function");

// Node crypto module shape
assert(typeof crypto.randomUUID === "function");
assert(typeof crypto.publicEncrypt === "function");
assert(crypto.crypto.subtle !== undefined);
assert(crypto.webcrypto.subtle !== undefined);
// Top-level subtle aliases webcrypto.subtle (Node-compatible).
assert(crypto.subtle === crypto.webcrypto.subtle);
// Named `crypto` property is the runtime global, not the module itself.
assert(typeof crypto.crypto.getRandomValues === "function");

const encrypted = crypto.publicEncrypt(
  {
    key: Buffer.alloc(0),
    oaepHash: "sha1",
    padding: crypto.constants.RSA_PKCS1_OAEP_PADDING,
  },
  Buffer.alloc(0)
);
assert(Buffer.isBuffer(encrypted));
assert(crypto.constants.RSA_PKCS1_OAEP_PADDING === 4);
