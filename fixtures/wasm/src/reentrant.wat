;; Imports a JS callback and exports a plain function; the JS callback
;; implementation calls back into `add` while wasmi is still executing
;; `triggerReentrant`, exercising the ActiveCaller reentrancy path.
(module
  (import "env" "hostCall" (func $hostCall (param i32) (result i32)))
  (func $add (export "add") (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.add)
  (func (export "triggerReentrant") (param $x i32) (result i32)
    local.get $x
    call $hostCall))
