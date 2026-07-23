;; Exports a small growable memory plus read/write/grow helpers, for testing
;; the bidirectional synchronous memory mirror (JS<->Wasm visibility) and
;; Wasm-internal `memory.grow`.
(module
  (memory (export "mem") 1 4)
  (func (export "writeI32") (param $offset i32) (param $value i32)
    local.get $offset
    local.get $value
    i32.store)
  (func (export "readI32") (param $offset i32) (result i32)
    local.get $offset
    i32.load)
  (func (export "growInternal") (param $delta i32) (result i32)
    local.get $delta
    memory.grow))
