import { promisify } from "util";
import { access, constants, lstat, stat, type Stats } from "fs";

// Direct callback overloads with optional mode/options must still typecheck.
access(".", () => {});
access(".", constants.R_OK, () => {});
stat(".", () => {});
stat(".", { bigint: false }, () => {});
lstat(".", () => {});
lstat(".", null, () => {});

// util.promisify uses the last overload (callback-only).
const accessAsync: (path: string) => Promise<void> = promisify(access);
const statAsync: (path: string) => Promise<Stats> = promisify(stat);
const lstatAsync: (path: string) => Promise<Stats> = promisify(lstat);

declare function assertPromiseVoid(p: Promise<void>): void;
declare function assertPromiseStats(p: Promise<Stats>): void;

assertPromiseVoid(accessAsync("."));
assertPromiseStats(statAsync("."));
assertPromiseStats(lstatAsync("."));
