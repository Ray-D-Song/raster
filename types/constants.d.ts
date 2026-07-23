/**
 * Legacy Node.js `constants` module (flat access-mode bits only).
 *
 * Open flags, errno, crypto, and signal constants are intentionally omitted.
 */
declare module "constants" {
  export const F_OK: 0;
  export const R_OK: 4;
  export const W_OK: 2;
  export const X_OK: 1;

  interface Constants {
    readonly F_OK: 0;
    readonly R_OK: 4;
    readonly W_OK: 2;
    readonly X_OK: 1;
  }

  const constants: Constants;
  export default constants;
}

declare module "node:constants" {
  export * from "constants";
  import constants from "constants";
  export default constants;
}
