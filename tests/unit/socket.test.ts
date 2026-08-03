import net from "node:net";

describe("createServer and connect", () => {
  it("should create a server and connect to it", (done) => {
    const server = net.createServer();
    server.listen(() => {
      const client = net.connect((server.address() as any).port, () => {
        client.end(() => {
          server.close(done);
        });
      });
    });
  });

  it("should handle data transfer between server and client", (done) => {
    const message = "Hello from client";
    const server = net.createServer((socket) => {
      socket.on("data", (data) => {
        expect(data.toString()).toEqual(message);
        socket.write(data);
      });
    });
    server.listen(() => {
      const client = net.connect((server.address() as any).port, () => {
        client.write(message);
        client.on("data", (data) => {
          expect(data.toString()).toEqual(message);
          client.end(() => {
            server.close(done);
          });
        });
      });
    });
  });

  it("should handle data from server first", (done) => {
    const message = "Hello from client";
    const server = net.createServer((socket) => {
      socket.write(message);
    });
    server.listen(() => {
      const client = net.connect((server.address() as any).port, () => {
        client.on("data", (data) => {
          expect(data.toString()).toEqual(message);
          client.end(() => {
            server.close(done);
          });
        });
      });
    });
  });
});

describe("error handling", () => {
  it("should handle connection error", (done) => {
    // Bind and immediately close a server to get a guaranteed-refused port
    const tmp = net.createServer();
    tmp.listen(0, () => {
      const port = (tmp.address() as any).port;
      tmp.close(() => {
        const client = net
          .connect(port, "127.0.0.1")
          .on("error", (error) => {
            expect(error).toBeInstanceOf(Error);
            client.destroy();
            done();
          });
      });
    });
  });

  it("should handle server destroy", (done) => {
    const server = net.createServer((socket) => {
      socket.on("data", () => {
        socket.destroy();
      });
    });

    server.listen(() => {
      const client = net.connect((server.address() as any).port, () => {
        client.write("hello");
      });
      client.on("close", () => {
        client.end();
        server.close(done);
      });
    });
  });

  it("should handle client destroy", (done) => {
    let closedResolve: () => void;
    const closePromise = new Promise<void>((resolve) => {
      closedResolve = resolve;
    });
    const server = net.createServer(async (socket) => {
      await closePromise;
      setTimeout(() => {
        socket.write("hello", (err) => {
          expect(err).toBeTruthy();
          server.close();
          done();
        });
      }, 5);
    });

    server.listen(() => {
      const client = net.connect((server.address() as any).port, () => {
        client.destroy();
        client.on("close", closedResolve);
      });
    });
  });
});

describe("socket readable/writable state", () => {
  it("starts readable and writable before connect", () => {
    const socket = new net.Socket();
    expect(socket.readable).toBe(true);
    expect(socket.writable).toBe(true);
    expect(socket.destroyed).toBe(false);
    expect(socket.readableEnded).toBe(false);
    expect(socket.writableEnded).toBe(false);
    socket.destroy();
    expect(socket.destroyed).toBe(true);
    expect(socket.readable).toBe(false);
    expect(socket.writable).toBe(false);
  });

  it("updates writableEnded after end()", (done) => {
    const server = net.createServer((socket) => {
      socket.on("data", () => {});
    });
    server.listen(() => {
      const client = net.connect((server.address() as any).port, () => {
        expect(client.readable).toBe(true);
        expect(client.writable).toBe(true);
        client.end(() => {
          expect(client.writable).toBe(false);
          expect(client.writableEnded).toBe(true);
          client.destroy();
          server.close(done);
        });
      });
    });
  });

  it("sets readableEnded after remote EOF", (done) => {
    const server = net.createServer((socket) => {
      socket.end("bye");
    });
    server.listen(() => {
      const client = net.connect((server.address() as any).port);
      client.on("end", () => {
        expect(client.readable).toBe(false);
        expect(client.readableEnded).toBe(true);
        client.destroy();
        server.close(done);
      });
    });
  });
});
