import defaultImport from "node:net";
import legacyImport from "net";

it("node:net should be the same as net", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

describe("net.isIP", () => {
  const { isIP } = defaultImport;

  it("classifies IPv4 and IPv6", () => {
    expect(isIP("127.0.0.1")).toBe(4);
    expect(isIP("::1")).toBe(6);
    expect(isIP("2001:db8::1")).toBe(6);
  });

  it("accepts IPv6 zone IDs", () => {
    expect(isIP("fe80::1%lo0")).toBe(6);
  });

  it("rejects illegal zone IDs (Node parity)", () => {
    // Empty zone, multiple zones, IPv4 with zone → 0
    expect(isIP("fe80::1%")).toBe(0);
    expect(isIP("fe80::1%a%b")).toBe(0);
    expect(isIP("127.0.0.1%x")).toBe(0);
    // Disallowed zone characters (Node returns 0)
    expect(isIP("fe80::1%a/b")).toBe(0);
    expect(isIP("fe80::1%a_b")).toBe(0);
    expect(isIP("fe80::1%🚀")).toBe(0);
    // Allowed zone charset: alnum, '.', ':', '-'
    expect(isIP("fe80::1%a:b")).toBe(6);
    expect(isIP("fe80::1%a.b")).toBe(6);
    expect(isIP("fe80::1%a-b")).toBe(6);
  });

  it("rejects hostnames, non-strings, empty, and whitespace", () => {
    expect(isIP("localhost")).toBe(0);
    expect(isIP("example.com")).toBe(0);
    expect(isIP("")).toBe(0);
    expect(isIP(" 127.0.0.1")).toBe(0);
    expect(isIP("127.0.0.1 ")).toBe(0);
    expect(isIP(null as any)).toBe(0);
    expect(isIP(undefined as any)).toBe(0);
    expect(isIP(4 as any)).toBe(0);
  });
});
