;; A `start` function that unconditionally traps; instantiation itself must
;; reject with a RuntimeError once imports are linked.
(module
  (func $bad
    unreachable)
  (start $bad))
