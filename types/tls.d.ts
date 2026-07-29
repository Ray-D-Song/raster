/**
 * The `tls` module provides an implementation of the Transport Layer Security
 * (TLS) and Secure Socket Layer (SSL) protocols.
 */
declare module "tls" {
  import { Buffer } from "buffer";
  import { EventEmitter } from "events";
  import { Socket, type SocketReadyState } from "net";

  interface TlsOptions {
    ca?: string | Buffer | Array<string | Buffer> | undefined;
    cert?: string | Buffer | Array<string | Buffer> | undefined;
    key?: string | Buffer | undefined;
    passphrase?: string | undefined;
    minVersion?: "TLSv1.2" | "TLSv1.3" | undefined;
    maxVersion?: "TLSv1.2" | "TLSv1.3" | undefined;
    ALPNProtocols?: string[] | Buffer | undefined;
    servername?: string | undefined;
    host?: string | undefined;
    port?: number | undefined;
    rejectUnauthorized?: boolean | undefined;
    requestCert?: boolean | undefined;
    checkServerIdentity?: ((hostname: string, cert: object) => Error | undefined) | undefined;
    secureContext?: SecureContext | undefined;
    timeout?: number | undefined;
    socket?: Socket | undefined;
  }

  interface ConnectionOptions extends TlsOptions {
    port?: number | undefined;
    host?: string | undefined;
    socket?: Socket | undefined;
  }

  class SecureContext {}

  class TLSSocket extends Socket {
    readonly encrypted: true;
    readonly secureConnecting: boolean;
    readonly authorized: boolean;
    readonly authorizationError: Error | undefined;
    readonly alpnProtocol: string | false;
    readonly servername: string | undefined;
    readonly connecting: boolean;
    readonly pending: boolean;
    readonly readyState: SocketReadyState;
    readonly localAddress?: string;
    readonly localPort?: number;
    readonly remoteAddress?: string;
    readonly remotePort?: number;

    write(
      chunk: string | Buffer,
      callback?: (err?: Error) => void,
    ): boolean;
    end(callback?: () => void): this;
    end(chunk?: string | Buffer, callback?: () => void): this;
    destroy(error?: Error): this;
    read(size?: number): Buffer | null;
    getPeerCertificate(detailed?: boolean): object;
    getCertificate(): object | null;
    getCipher(): { name: string; standardName: string; version: string } | null;
    getProtocol(): string | null;
    isSessionReused(): boolean;

    on(event: "connect", listener: () => void): this;
    on(event: "secure", listener: () => void): this;
    on(event: "secureConnect", listener: () => void): this;
    on(event: "error", listener: (err: Error) => void): this;
    on(event: "close", listener: (hadError: boolean) => void): this;
    on(event: "data", listener: (data: Buffer) => void): this;
    on(event: "end", listener: () => void): this;
    on(event: "readable", listener: () => void): this;
    on(event: "finish", listener: () => void): this;
    on(event: string | symbol, listener: (...args: unknown[]) => void): this;
  }

  class Server extends EventEmitter {
    listen(
      port?: number,
      hostname?: string,
      callback?: () => void,
    ): this;
    listen(options: { port?: number; host?: string }, callback?: () => void): this;
    close(callback?: () => void): this;
    address(): { port: number; family: string; address: string } | string | null;
    getConnections(callback: (err: Error | null, count: number) => void): void;
    setSecureContext(context: SecureContext): void;
    addContext(hostname: string, context: SecureContext): void;

    on(event: "secureConnection", listener: (socket: TLSSocket) => void): this;
    on(event: "tlsClientError", listener: (err: Error, tlsSocket: TLSSocket) => void): this;
    on(event: "connection", listener: (socket: TLSSocket) => void): this;
    on(event: "close", listener: () => void): this;
    on(event: string | symbol, listener: (...args: unknown[]) => void): this;
  }

  function connect(
    options: ConnectionOptions,
    callback?: () => void,
  ): TLSSocket;
  function connect(
    port: number,
    host?: string,
    options?: ConnectionOptions,
    callback?: () => void,
  ): TLSSocket;
  function connect(
    port: number,
    options?: ConnectionOptions,
    callback?: () => void,
  ): TLSSocket;
  function connect(
    port: number,
    callback?: () => void,
  ): TLSSocket;

  function createServer(
    options?: TlsOptions,
    secureConnectionListener?: (socket: TLSSocket) => void,
  ): Server;

  function createSecureContext(options?: TlsOptions): SecureContext;

  function checkServerIdentity(
    hostname: string,
    cert: object,
  ): Error | undefined;

  function getCiphers(): string[];

  function convertALPNProtocols(
    protocols: string[] | Buffer,
    out: { ALPNProtocols?: Buffer },
  ): void;

  const CLIENT_RENEG_LIMIT: number;
  const CLIENT_RENEG_WINDOW: number;
  const DEFAULT_CIPHERS: string;
  const DEFAULT_ECDH_CURVE: string;
  const DEFAULT_MIN_VERSION: string;
  const DEFAULT_MAX_VERSION: string;

  export {
    TLSSocket,
    Server,
    SecureContext,
    connect,
    createServer,
    createSecureContext,
    checkServerIdentity,
    getCiphers,
    convertALPNProtocols,
    CLIENT_RENEG_LIMIT,
    CLIENT_RENEG_WINDOW,
    DEFAULT_CIPHERS,
    DEFAULT_ECDH_CURVE,
    DEFAULT_MIN_VERSION,
    DEFAULT_MAX_VERSION,
  };

  interface TlsModule {
    TLSSocket: typeof TLSSocket;
    Server: typeof Server;
    SecureContext: typeof SecureContext;
    connect: typeof connect;
    createServer: typeof createServer;
    createSecureContext: typeof createSecureContext;
    checkServerIdentity: typeof checkServerIdentity;
    getCiphers: typeof getCiphers;
    convertALPNProtocols: typeof convertALPNProtocols;
    CLIENT_RENEG_LIMIT: typeof CLIENT_RENEG_LIMIT;
    CLIENT_RENEG_WINDOW: typeof CLIENT_RENEG_WINDOW;
    DEFAULT_CIPHERS: typeof DEFAULT_CIPHERS;
    DEFAULT_ECDH_CURVE: typeof DEFAULT_ECDH_CURVE;
    DEFAULT_MIN_VERSION: typeof DEFAULT_MIN_VERSION;
    DEFAULT_MAX_VERSION: typeof DEFAULT_MAX_VERSION;
  }
  const tls: TlsModule;
  export default tls;
}

declare module "node:tls" {
  export * from "tls";
  export { default } from "tls";
}
