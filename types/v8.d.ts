/**
 * Node-compatible `v8` surface backed by QuickJS heap statistics.
 *
 * Field names follow Node's snake_case. Values are derived from QuickJS
 * `JS_ComputeMemoryUsage`, not Google V8. Serialization, snapshots, and
 * profilers are not implemented. `setFlagsFromString` is a compatibility no-op.
 */
declare module "v8" {
  export interface HeapStatistics {
    total_heap_size: number;
    total_heap_size_executable: number;
    total_physical_size: number;
    total_available_size: number;
    used_heap_size: number;
    heap_size_limit: number;
    malloced_memory: number;
    peak_malloced_memory: number;
    does_zap_garbage: number;
    number_of_native_contexts: number;
    number_of_detached_contexts: number;
    total_global_handles_size: number;
    used_global_handles_size: number;
    external_memory: number;
    total_allocated_bytes: number;
  }

  export interface HeapSpaceStatistics {
    space_name: string;
    space_size: number;
    space_used_size: number;
    space_available_size: number;
    physical_space_size: number;
  }

  export interface HeapCodeStatistics {
    code_and_metadata_size: number;
    bytecode_and_metadata_size: number;
    external_script_source_size: number;
    cpu_profiler_metadata_size: number;
  }

  export function getHeapStatistics(): HeapStatistics;
  export function getHeapSpaceStatistics(): HeapSpaceStatistics[];
  export function getHeapCodeStatistics(): HeapCodeStatistics;
  /**
   * Compatibility no-op. Accepts a flags string and returns `undefined`.
   * Flags are not applied to QuickJS.
   */
  export function setFlagsFromString(flags: string): void;
}

declare module "node:v8" {
  export * from "v8";
  import v8 from "v8";
  export default v8;
}
