import {
  parentPort,
  workerData,
  getEnvironmentData,
} from "node:worker_threads";

parentPort.on("message", (msg) => {
  if (msg?.type === "clone-check") {
    const { when, tags } = msg.payload;
    parentPort.postMessage({
      ok: true,
      when: when instanceof Date ? when.toISOString() : String(when),
      mapSize: tags instanceof Map ? tags.size : -1,
    });
    return;
  }
  if (workerData?.askEnv) {
    const env = getEnvironmentData("raster-worker-env-key");
    parentPort.postMessage({ envV: env?.v });
    return;
  }
  parentPort.postMessage({ pong: msg.ping, workerData });
});
