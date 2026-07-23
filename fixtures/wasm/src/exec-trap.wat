;; Exported function that traps during normal execution (division by zero),
;; for exercising the RuntimeError path for ordinary Wasm traps (as opposed
;; to a trapping `start` function).
(module
  (func (export "divide") (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.div_s))
