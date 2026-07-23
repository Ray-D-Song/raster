;; Declares a shared memory (requires the threads proposal). Out of scope
;; for this batch: must be rejected with a CompileError at the pre-check
;; stage, not passed through to wasmi.
(module
  (memory (export "mem") 1 4 shared))
