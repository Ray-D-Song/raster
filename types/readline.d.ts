/**
 * The `node:readline` module provides an interface for reading data from a Readable stream (such as `process.stdin`) one line at a time.
 * @see https://nodejs.org/docs/latest-v24.x/api/readline.html
 */
declare module "readline" {
  import { EventEmitter } from "events";
  import * as promises from "readline/promises";

  interface Key {
    sequence?: string | undefined;
    name?: string | undefined;
    ctrl?: boolean | undefined;
    meta?: boolean | undefined;
    shift?: boolean | undefined;
  }

  interface InterfaceOptions {
    input: NodeJS.ReadableStream;
    output?: NodeJS.WritableStream | undefined;
    completer?: Completer | AsyncCompleter | undefined;
    terminal?: boolean | undefined;
    history?: string[] | undefined;
    historySize?: number | undefined;
    removeHistoryDuplicates?: boolean | undefined;
    prompt?: string | undefined;
    crlfDelay?: number | undefined;
    escapeCodeTimeout?: number | undefined;
    tabSize?: number | undefined;
    signal?: AbortSignal | undefined;
  }

  type Completer = (line: string) => CompleterResult;
  type AsyncCompleter = (
    line: string,
    callback: (err?: null | Error, result?: CompleterResult) => void
  ) => void;
  type CompleterResult = [string[], string];

  interface ReadLineOptions extends InterfaceOptions {}

  class Interface extends EventEmitter {
    readonly terminal: boolean;
    readonly line: string;
    readonly cursor: number;
    constructor(
      input: NodeJS.ReadableStream,
      output?: NodeJS.WritableStream,
      completer?: Completer | AsyncCompleter,
      terminal?: boolean
    );
    constructor(options: InterfaceOptions);
    getPrompt(): string;
    setPrompt(prompt: string): void;
    prompt(preserveCursor?: boolean): void;
    question(query: string, callback: (answer: string) => void): void;
    question(
      query: string,
      options: { signal?: AbortSignal },
      callback: (answer: string) => void
    ): void;
    pause(): this;
    resume(): this;
    close(): void;
    write(data: string | Buffer, key?: Key): void;
    getCursorPos(): { rows: number; cols: number };
    [Symbol.asyncIterator](): AsyncIterableIterator<string>;
    [Symbol.dispose](): void;
  }

  function createInterface(
    input: NodeJS.ReadableStream,
    output?: NodeJS.WritableStream,
    completer?: Completer | AsyncCompleter,
    terminal?: boolean
  ): Interface;
  function createInterface(options: InterfaceOptions): Interface;

  function emitKeypressEvents(
    stream: NodeJS.ReadableStream,
    readlineInterface?: Interface
  ): void;

  function clearLine(
    stream: NodeJS.WritableStream,
    dir: -1 | 0 | 1,
    callback?: () => void
  ): boolean;
  function clearScreenDown(
    stream: NodeJS.WritableStream,
    callback?: () => void
  ): boolean;
  function cursorTo(
    stream: NodeJS.WritableStream,
    x: number,
    y?: number,
    callback?: () => void
  ): boolean;
  function moveCursor(
    stream: NodeJS.WritableStream,
    dx: number,
    dy: number,
    callback?: () => void
  ): boolean;

  const promises: typeof import("readline/promises");

  export {
    Interface,
    createInterface,
    clearLine,
    clearScreenDown,
    cursorTo,
    moveCursor,
    emitKeypressEvents,
    promises,
    Completer,
    AsyncCompleter,
    CompleterResult,
    Key,
    InterfaceOptions,
    ReadLineOptions,
  };
  export default {
    Interface,
    createInterface,
    clearLine,
    clearScreenDown,
    cursorTo,
    moveCursor,
    emitKeypressEvents,
    promises,
  };
}

declare module "node:readline" {
  export * from "readline";
  export { default } from "readline";
}
