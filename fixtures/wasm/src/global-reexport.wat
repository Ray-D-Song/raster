;; Imports a mutable i32 global and re-exports it under a different name, to
;; verify that an imported Global keeps its JS wrapper identity across the
;; import/export boundary.
(module
  (import "env" "counter" (global $counter (mut i32)))
  (export "counterAlias" (global $counter)))
