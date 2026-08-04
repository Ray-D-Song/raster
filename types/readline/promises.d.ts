/**
 * Promise-based `node:readline/promises` API (Node 24.3).
 */
declare module "readline/promises" {
  import { Interface as CallbackInterface, InterfaceOptions, Key } from "readline";

  class Interface extends CallbackInterface {
    question(query: string, options?: { signal?: AbortSignal }): Promise<string>;
  }

  class Readline {
    constructor(
      stream: NodeJS.WritableStream,
      options?: { autoCommit?: boolean }
    );
    cursorTo(x: number, y?: number): this;
    moveCursor(dx: number, dy: number): this;
    clearLine(dir: -1 | 0 | 1): this;
    clearScreenDown(): this;
    commit(): Promise<void>;
    rollback(): this;
  }

  function createInterface(
    input: NodeJS.ReadableStream,
    output?: NodeJS.WritableStream,
    completer?: any,
    terminal?: boolean
  ): Interface;
  function createInterface(options: InterfaceOptions): Interface;

  export { Interface, Readline, createInterface, Key };
  export default { Interface, Readline, createInterface };
}

declare module "node:readline/promises" {
  export * from "readline/promises";
  export { default } from "readline/promises";
}
