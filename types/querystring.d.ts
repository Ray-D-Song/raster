declare module "querystring" {
  export interface StringifyOptions {
    encodeURIComponent?: (value: string) => string;
  }
  export interface ParseOptions {
    maxKeys?: number;
    decodeURIComponent?: (value: string) => string;
  }
  export function stringify(
    obj?: Record<string, unknown>,
    sep?: string,
    eq?: string,
    options?: StringifyOptions
  ): string;
  export function parse(
    str: string,
    sep?: string,
    eq?: string,
    options?: ParseOptions
  ): Record<string, string | string[]>;
  export function escape(value: string): string;
  export function unescape(value: string): string;
  export const encode: typeof stringify;
  export const decode: typeof parse;

  interface QuerystringModule {
    stringify: typeof stringify;
    parse: typeof parse;
    escape: typeof escape;
    unescape: typeof unescape;
    encode: typeof stringify;
    decode: typeof parse;
  }
  const querystring: QuerystringModule;
  export default querystring;
}
declare module "node:querystring" {
  export * from "querystring";
  export { default } from "querystring";
}
