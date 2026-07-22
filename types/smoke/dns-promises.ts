import lookupNamed, { lookup as namedLookup } from "dns/promises";
import lookupNode from "node:dns/promises";
import type { LookupAddress, LookupAllOptions, LookupOptions } from "dns";

declare function assertLookup(
  p: Promise<LookupAddress | LookupAddress[]>
): void;
declare function assertLookupOne(p: Promise<LookupAddress>): void;
declare function assertLookupAll(p: Promise<LookupAddress[]>): void;

assertLookupOne(namedLookup("example.com"));
assertLookupOne(lookupNamed.lookup("example.com", 4));
assertLookupAll(lookupNode.lookup("example.com", { all: true }));

const allOpts: LookupAllOptions = { all: true };
assertLookupAll(namedLookup("example.com", allOpts));

const dynamic: LookupOptions = { all: true };
assertLookup(namedLookup("example.com", dynamic));
