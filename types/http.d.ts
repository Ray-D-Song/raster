declare module "http" {
  import { EventEmitter } from "events";
  import { Buffer } from "buffer";

  export interface AddressInfo { address: string; family: string; port: number; }
  export interface ListenOptions { port?: number; host?: string; path?: string; }
  export interface ServerOptions {
    insecureHTTPParser?: boolean;
    maxHeaderSize?: number;
    maxBodySize?: number;
    maxHeadersCount?: number;
    headersTimeout?: number;
    requestTimeout?: number;
    keepAliveTimeout?: number;
  }

  export class IncomingMessage extends EventEmitter {
    method: string;
    url: string;
    httpVersion: "1.1";
    headers: Record<string, string | string[]>;
    rawHeaders: string[];
    trailers: Record<string, string>;
    rawTrailers: string[];
    complete: boolean;
    socket: import("net").Socket;
    read(): Buffer;
    pause(): this;
    resume(): this;
    destroy(): this;
  }

  export class ServerResponse extends EventEmitter {
    statusCode: number;
    statusMessage: string;
    readonly headersSent: boolean;
    setHeader(name: string, value: string): void;
    getHeader(name: string): string | undefined;
    getHeaders(): Record<string, string>;
    hasHeader(name: string): boolean;
    removeHeader(name: string): void;
    writeHead(statusCode: number, headers?: Record<string, string>): this;
    writeHead(statusCode: number, statusMessage: string, headers?: Record<string, string>): this;
    flushHeaders(): void;
    addTrailers(trailers: Record<string, string>): void;
    writeContinue(): void;
    write(chunk: string | Buffer): boolean;
    end(chunk?: string | Buffer): void;
  }

  export class Server extends EventEmitter {
    readonly listening: boolean;
    maxHeadersCount: number;
    headersTimeout: number;
    requestTimeout: number;
    keepAliveTimeout: number;
    listen(port?: number, host?: string, callback?: () => void): this;
    listen(path: string, callback?: () => void): this;
    listen(options: ListenOptions, callback?: () => void): this;
    address(): AddressInfo | undefined;
    close(callback?: () => void): this;
    getConnections(callback: (error: Error | undefined, count: number) => void): void;
    closeAllConnections(): void;
    closeIdleConnections(): void;
    on(event: "request" | "checkContinue", listener: (request: IncomingMessage, response: ServerResponse) => void): this;
    on(event: "upgrade" | "connect", listener: (request: IncomingMessage, socket: import("net").Socket, head: Buffer) => void): this;
    on(event: "clientError", listener: (error: Error) => void): this;
  }

  export function createServer(listener?: (request: IncomingMessage, response: ServerResponse) => void): Server;
  export function createServer(options: ServerOptions, listener?: (request: IncomingMessage, response: ServerResponse) => void): Server;
  export const METHODS: string[];
  export const STATUS_CODES: Record<number, string>;
  export function validateHeaderName(name: string): void;
  export function validateHeaderValue(name: string, value: string): void;
}

declare module "node:http" { export * from "http"; }
