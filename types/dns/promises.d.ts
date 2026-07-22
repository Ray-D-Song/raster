/**
 * Promise-based DNS lookup APIs.
 */
declare module "dns/promises" {
  import type {
    LookupAddress,
    LookupAllOptions,
    LookupOneOptions,
    LookupOptions,
  } from "dns";

  export function lookup(
    hostname: string,
    options: LookupAllOptions
  ): Promise<LookupAddress[]>;
  export function lookup(
    hostname: string,
    options?: LookupOneOptions | number
  ): Promise<LookupAddress>;
  export function lookup(
    hostname: string,
    options: LookupOptions
  ): Promise<LookupAddress | LookupAddress[]>;

  const promises: {
    lookup: typeof lookup;
  };

  export default promises;
}

declare module "node:dns/promises" {
  export * from "dns/promises";
  export { default } from "dns/promises";
}
