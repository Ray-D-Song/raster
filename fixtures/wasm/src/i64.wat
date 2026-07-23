;; i64 <-> BigInt round trip: adds 1 (as i64) to the argument, and forwards
;; to a JS import that must accept/return BigInt as well.
(module
  (import "env" "incrHost" (func $incrHost (param i64) (result i64)))
  (func (export "incr") (param $x i64) (result i64)
    local.get $x
    i64.const 1
    i64.add)
  (func (export "callIncrHost") (param $x i64) (result i64)
    local.get $x
    call $incrHost))
