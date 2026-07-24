/**
 * The `timers/promises` API provides an alternative set of timer functions
 * that return `Promise` objects.
 *
 * Raster supports `setTimeout` and `setImmediate`. `setInterval` (async iterator)
 * and `scheduler` are not implemented.
 *
 * Options may include `{ signal?: AbortSignal, ref?: boolean }`. `ref: false` is
 * accepted for API compatibility but does not yet change event-loop lifetime.
 */
declare module "timers/promises" {
  interface TimerOptions {
    /**
     * Accepted for Node compatibility. Raster does not yet honor unref semantics.
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
