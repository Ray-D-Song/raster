/**
 * The `timers/promises` API provides an alternative set of timer functions
 * that return `Promise` objects.
 *
 * Raster supports `setTimeout` and `setImmediate`. `setInterval` (async iterator)
 * and `scheduler` are not implemented.
 *
 * Options may include `{ signal?: AbortSignal, ref?: boolean }`. `ref: false` keeps
 * the scheduled timer from holding the event loop open by itself.
 */
declare module "timers/promises" {
  interface TimerOptions {
    /**
     * When `false`, the timer is unref'd and does not keep the process alive alone.
     * @default true
     */
    ref?: boolean;
    /**
     * An optional `AbortSignal` that can be used to cancel the scheduled timer.
     */
    signal?: AbortSignal;
  }

  /**
   * @param delay The number of milliseconds to wait before fulfilling the promise.
   * @param value A value with which the promise is fulfilled.
   */
  function setTimeout<T = void>(
    delay?: number,
    value?: T,
    options?: TimerOptions
  ): Promise<T>;

  /**
   * @param value A value with which the promise is fulfilled.
   */
  function setImmediate<T = void>(
    value?: T,
    options?: TimerOptions
  ): Promise<T>;
}

declare module "node:timers/promises" {
  export * from "timers/promises";
}
