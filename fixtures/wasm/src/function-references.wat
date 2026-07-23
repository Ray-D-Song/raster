;; Uses a typed function reference (`(ref func)`) from the
;; function-references proposal. Out of scope for this batch: must be
;; rejected with a CompileError.
(module
  (type $ft (func (param i32) (result i32)))
  (func $inc (type $ft) (param $x i32) (result i32)
    local.get $x
    i32.const 1
    i32.add)
  (elem declare func $inc)
  (func (export "getRef") (result (ref $ft))
    ref.func $inc))
