;; Declares an `i64`-addressed ("memory64") memory. Out of scope for this
;; batch: must be rejected with a CompileError.
(module
  (memory (export "mem") i64 1))
