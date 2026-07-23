;; Exports a function whose *result* is v128 directly, so that calling it
;; from JS must throw a TypeError (v128 cannot cross the JS/Wasm boundary).
(module
  (func (export "makeVector") (param $x i32) (result v128)
    local.get $x
    i32x4.splat))
