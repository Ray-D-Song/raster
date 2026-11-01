import { parentPort, workerData } from "node:worker_threads";

parentPort.on("message", (msg) => {
  parentPort.postMessage({ pong: msg.ping, workerData });
});
