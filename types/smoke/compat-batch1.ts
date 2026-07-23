import { chdir, pid } from "node:process";
import { getHeapStatistics } from "node:v8";
import { F_OK } from "node:constants";

declare function assert(condition: boolean): void;

assert(typeof pid === "number");
assert(typeof chdir === "function");
assert(typeof getHeapStatistics === "function");
assert(F_OK === 0);
