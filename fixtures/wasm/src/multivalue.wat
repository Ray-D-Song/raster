;; Multi-result export (swap), and a multi-result *import* whose JS callback
;; must supply enough array elements, exercising both directions of the
;; multi-value proposal.
(module
  (import "env" "divmod" (func $divmod (param i32 i32) (result i32 i32)))
  (func (export "swap") (param $a i32) (param $b i32) (result i32 i32)
    local.get $b
    local.get $a)
  (func (export "callDivmod") (param $a i32) (param $b i32) (result i32 i32)
    local.get $a
    local.get $b
    call $divmod))
