process.on("message", (msg) => {
  if (msg?.ping != null) {
    process.send({ pong: msg.ping });
  }
});
