;; Declares an exception-handling `tag`. Out of scope for this batch: must
;; be rejected with a CompileError.
(module
  (tag $e (param i32)))
