
declare module "tty" {
  import { EventEmitter } from "events";

  /**
   * The `tty.isatty()` method returns `true` if the given `fd` is associated with
   * a TTY and `false` if it is not, including whenever `fd` is not a non-negative
   * integer.
   * @since v0.5.8
   * @param fd A numeric file descriptor
   */
  function isatty(fd: number): boolean;

  class ReadStream extends EventEmitter {
    constructor(fd: number, options?: object);
    isRaw: boolean;
    isTTY: boolean;
    fd: number;
    setRawMode(mode: boolean): this;
    resume(): this;
    pause(): this;
  }

  class WriteStream extends EventEmitter {
    constructor(fd: number);
    isTTY: boolean;
    fd: number;
    columns: number;
    rows: number;
    write(
      chunk: string | Uint8Array,
      encodingOrCb?: string | ((err?: Error | null) => void),
      cb?: (err?: Error | null) => void
    ): boolean;
    getWindowSize(): [number, number];
    cursorTo(x: number, y?: number, callback?: () => void): boolean;
    moveCursor(dx: number, dy: number, callback?: () => void): boolean;
    clearLine(dir?: -1 | 0 | 1, callback?: () => void): boolean;
    clearScreenDown(callback?: () => void): boolean;
    hasColors(count?: number): boolean;
    getColorDepth(env?: object): number;
  }

  export { isatty, ReadStream, WriteStream };
}

declare module "node:tty" {
  export * from "tty";
}