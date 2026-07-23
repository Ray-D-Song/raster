;; Writes to linear memory, then calls a JS import while that write is still
;; only visible through the synchronous memory mirror -- exercises the
;; Wasm->JS mirror sync that must happen before entering a host callback.
(module
  (import "env" "onWrite" (func $onWrite))
  (memory (export "mem") 1)
  (func (export "writeThenCallback") (param $offset i32) (param $value i32)
    local.get $offset
    local.get $value
    i32.store
    call $onWrite))
