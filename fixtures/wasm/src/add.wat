;; Trivial exported function, reused for basic compile/instantiate tests and
;; for verifying that re-exporting the same extern preserves wrapper identity
;; (see webassembly.test.ts: "add" and "addAlias" both point at $add).
(module
  (func $add (export "add") (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.add)
  (export "addAlias" (func $add))
  (global $answer (export "answer") i32 (i32.const 42))
  (memory (export "mem") 1 2))
