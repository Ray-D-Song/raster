;; Imports a single JS function and calls it from an exported function; used
;; for JS-function-import tests, callback getter/Proxy access order tests,
;; and callback-thrown-value identity tests.
(module
  (import "env" "log" (func $log (param i32)))
  (func (export "run") (param $x i32)
    local.get $x
    call $log))
