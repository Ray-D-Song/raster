import qs from "node:querystring";
import legacy from "querystring";
import Module from "node:module";

const { parse, stringify, escape, unescape, encode, decode } = qs;

it("registers querystring as a builtin module", () => {
  expect(Module.isBuiltin("querystring")).toBe(true);
  expect(require("querystring")).toBe(qs);
  expect(require("node:querystring")).toBe(qs);
  expect(legacy).toBe(qs);
});

it("stringifies scalars, arrays, and repeated keys", () => {
  expect(stringify({ a: 1, b: ["x", "y"] })).toBe("a=1&b=x&b=y");
  expect(stringify({ empty: [] })).toBe("");
  expect(parse("a=1&b=x&b=y")).toEqual({ a: "1", b: ["x", "y"] });
});

it("supports custom separators and encoders", () => {
  expect(stringify({ a: "b" }, ";", ":")).toBe("a:b");
  expect(stringify({ a: 1, b: 2 }, "", ":")).toBe("a:1&b:2");
  expect(parse("a:b", ";", ":", { maxKeys: 0 })).toEqual({ a: "b" });
});

it("aliases encode/decode to stringify/parse", () => {
  expect(encode).toBe(stringify);
  expect(decode).toBe(parse);
  expect(encode({ a: 1 })).toBe("a=1");
  expect(decode("a=1")).toEqual({ a: "1" });
});

it("handles escape, unescape, and malformed percent sequences", () => {
  expect(escape("a b!")).toBe("a%20b!");
  expect(unescape("a+b%2Zc")).toBe("a b%2Zc");
  expect(unescape("%E2%28")).toBe("\uFFFD(");
  expect(() => escape("\uD800")).toThrow(URIError);
});
