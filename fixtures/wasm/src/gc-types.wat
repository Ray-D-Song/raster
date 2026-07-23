;; Declares a GC struct type. Out of scope for this batch: must be rejected
;; with a CompileError even though the type is never instantiated.
(module
  (type $point (struct (field i32) (field i32))))
