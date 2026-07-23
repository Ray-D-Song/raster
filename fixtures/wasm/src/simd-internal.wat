;; Uses SIMD (v128) purely internally: builds an i32x4 lane vector, adds it
;; to itself, and extracts a lane back out as a plain i32 result. Verifies
;; that SIMD modules compile/instantiate/execute without ever exposing a
;; v128 value across the JS/Wasm boundary.
(module
  (func (export "simdDouble") (param $x i32) (result i32)
    (local $v v128)
    local.get $x
    i32x4.splat
    local.set $v
    local.get $v
    local.get $v
    i32x4.add
    i32x4.extract_lane 0))
