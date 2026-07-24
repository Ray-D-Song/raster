/**
 * Minimal Node Inspector probe surface.
 *
 * Raster does **not** implement the Node Inspector debugging protocol.
 * Only `url()` is exported; it always returns `undefined`.
 */
declare module "inspector" {
  /**
   * Returns the active inspector URL, or `undefined` if none is listening.
   * Raster always returns `undefined`.
   */
  function url(): string | undefined;
}
declare module "node:inspector" {
  export * from "inspector";
}
