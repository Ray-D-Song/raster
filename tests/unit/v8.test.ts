import Module from "node:module";
import v8 from "node:v8";
import legacy from "v8";
import {
  getHeapCodeStatistics,
  getHeapSpaceStatistics,
  getHeapStatistics,
  setFlagsFromString,
} from "node:v8";

it("node:v8 should match v8", () => {
  expect(v8.getHeapStatistics).toBe(legacy.getHeapStatistics);
});

it("Module.isBuiltin recognizes v8", () => {
  expect(Module.isBuiltin("v8")).toBe(true);
  expect(Module.isBuiltin("node:v8")).toBe(true);
  expect(Module.builtinModules).toContain("v8");
});

function assertSafeNumber(v: unknown, label: string) {
  expect(typeof v).toBe("number");
  expect(Number.isFinite(v as number)).toBeTruthy();
  expect(v as number).toBeGreaterThanOrEqual(0);
  expect(v as number).toBeLessThanOrEqual(Number.MAX_SAFE_INTEGER);
  if ((v as number) < 0) {
    throw new Error(`${label} negative`);
  }
}

it("getHeapStatistics returns finite non-negative fields", () => {
  const stats = getHeapStatistics();
  const keys = [
    "total_heap_size",
    "total_heap_size_executable",
    "total_physical_size",
    "total_available_size",
    "used_heap_size",
    "heap_size_limit",
    "malloced_memory",
    "peak_malloced_memory",
    "does_zap_garbage",
    "number_of_native_contexts",
    "number_of_detached_contexts",
    "total_global_handles_size",
    "used_global_handles_size",
    "external_memory",
    "total_allocated_bytes",
  ] as const;

  for (const key of keys) {
    assertSafeNumber(stats[key], key);
  }

  expect(stats.heap_size_limit).toBeGreaterThanOrEqual(stats.used_heap_size);
  expect(stats.total_available_size).toBeGreaterThanOrEqual(0);
  // Unconfigured QuickJS limit must not report as 0.
  expect(stats.heap_size_limit).toBeGreaterThan(0);
});

it("getHeapSpaceStatistics returns a single quickjs space", () => {
  const spaces = getHeapSpaceStatistics();
  expect(Array.isArray(spaces)).toBeTruthy();
  expect(spaces.length).toBe(1);
  expect(spaces[0].space_name).toBe("quickjs");
  assertSafeNumber(spaces[0].space_size, "space_size");
  assertSafeNumber(spaces[0].space_used_size, "space_used_size");
  assertSafeNumber(spaces[0].space_available_size, "space_available_size");
  assertSafeNumber(spaces[0].physical_space_size, "physical_space_size");
});

it("getHeapCodeStatistics returns four safe fields", () => {
  const code = getHeapCodeStatistics();
  assertSafeNumber(code.code_and_metadata_size, "code_and_metadata_size");
  assertSafeNumber(code.bytecode_and_metadata_size, "bytecode_and_metadata_size");
  assertSafeNumber(code.external_script_source_size, "external_script_source_size");
  assertSafeNumber(code.cpu_profiler_metadata_size, "cpu_profiler_metadata_size");
});

it("setFlagsFromString is a no-op for strings and TypeError otherwise", () => {
  expect(setFlagsFromString("--example")).toBeUndefined();
  expect(() => setFlagsFromString(42 as unknown as string)).toThrow(TypeError);
  expect(() => (setFlagsFromString as Function)()).toThrow(TypeError);
});
