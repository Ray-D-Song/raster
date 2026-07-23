;; A module shaped like Undici's llhttp Wasm build: an `env` import
;; namespace with 8 JS callbacks (the llhttp `on_*` parser hooks), exported
;; `memory`, a `table`, a `malloc`-like allocator export, and an `execute`
;; entry point that reads from linear memory and invokes several of the
;; callbacks. This exercises the *shape* of the real integration (many
;; imports + shared memory access from callbacks); it is not a copy of, or
;; special-cased for, the real llhttp binary.
(module
  (import "env" "on_message_begin" (func $on_message_begin (param i32) (result i32)))
  (import "env" "on_url" (func $on_url (param i32 i32 i32) (result i32)))
  (import "env" "on_status" (func $on_status (param i32 i32 i32) (result i32)))
  (import "env" "on_header_field" (func $on_header_field (param i32 i32 i32) (result i32)))
  (import "env" "on_header_value" (func $on_header_value (param i32 i32 i32) (result i32)))
  (import "env" "on_headers_complete" (func $on_headers_complete (param i32) (result i32)))
  (import "env" "on_body" (func $on_body (param i32 i32 i32) (result i32)))
  (import "env" "on_message_complete" (func $on_message_complete (param i32) (result i32)))

  (memory (export "memory") 2 16)
  (table (export "table") 1 8 funcref)

  (global $heapTop (mut i32) (i32.const 0))

  ;; Bump allocator, just enough to hand callbacks a scratch memory region.
  (func (export "malloc") (param $size i32) (result i32)
    (local $ptr i32)
    global.get $heapTop
    local.set $ptr
    global.get $heapTop
    local.get $size
    i32.add
    global.set $heapTop
    local.get $ptr)

  ;; Pretends to "parse" the buffer at [ptr, ptr+len) by writing a byte into
  ;; it and then driving the parser callbacks in llhttp's usual order,
  ;; reading the just-written byte back out of memory inside a callback.
  (func (export "execute") (param $parser i32) (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr
    i32.const 71 ;; 'G' of "GET"
    i32.store8

    local.get $parser
    call $on_message_begin
    drop

    local.get $ptr
    local.get $ptr
    local.get $len
    call $on_url
    drop

    local.get $parser
    call $on_headers_complete
    drop

    local.get $ptr
    local.get $ptr
    local.get $len
    call $on_body
    drop

    local.get $parser
    call $on_message_complete))
