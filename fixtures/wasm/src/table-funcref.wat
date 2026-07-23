;; Exports two distinct functions (for funcref table identity tests) and a
;; funcref table pre-populated with one of them via an elem segment.
(module
  (func $inc (export "inc") (param $x i32) (result i32)
    local.get $x
    i32.const 1
    i32.add)
  (func $dec (export "dec") (param $x i32) (result i32)
    local.get $x
    i32.const 1
    i32.sub)
  (table (export "funcs") 2 4 funcref)
  (elem (i32.const 0) $inc))
