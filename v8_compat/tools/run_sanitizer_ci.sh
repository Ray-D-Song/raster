#!/usr/bin/env bash
# Run Rust/C++/addon compat checks under a single sanitizer on Linux CI.
set -euo pipefail

SANITIZER="${1:?usage: run_sanitizer_ci.sh <address|undefined>}"
if [[ "${SANITIZER}" != "address" && "${SANITIZER}" != "undefined" ]]; then
  echo "unsupported sanitizer: ${SANITIZER}" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

TARGET="${TARGET:-x86_64-unknown-linux-gnu}"
if [[ "${SANITIZER}" == "address" ]]; then
  TOOLCHAIN="${TOOLCHAIN:-+nightly}"
else
  TOOLCHAIN="${TOOLCHAIN:-}"
fi
export CARGO_TARGET_DIR="${ROOT}/target-sanitizer-${SANITIZER}"
if [[ -n "${TOOLCHAIN}" ]]; then
  CARGO=(env -u CARGO_TARGET_DIR CARGO_TARGET_DIR="${CARGO_TARGET_DIR}" cargo "${TOOLCHAIN}")
else
  CARGO=(env -u CARGO_TARGET_DIR CARGO_TARGET_DIR="${CARGO_TARGET_DIR}" cargo)
fi

V8_HELLO_ADDON="${ROOT}/compat/v8-hello/build/Release/v8_hello.node"
NAPI_HELLO_ADDON="${ROOT}/compat/napi-hello/build/Release/hello.node"
BETTER_SQLITE3_ADDON="${ROOT}/compat/better-sqlite3/node_modules/better-sqlite3/build/Release/better_sqlite3.node"

unset RASTER_SANITIZE
unset RUSTFLAGS
unset CFLAGS
unset CXXFLAGS
unset LDFLAGS
unset CC
unset CXX

if [[ "${SANITIZER}" == "address" ]]; then
  if command -v llvm-symbolizer >/dev/null 2>&1; then
    export ASAN_SYMBOLIZER_PATH
    ASAN_SYMBOLIZER_PATH="$(command -v llvm-symbolizer)"
  elif command -v llvm-symbolizer-18 >/dev/null 2>&1; then
    export ASAN_SYMBOLIZER_PATH
    ASAN_SYMBOLIZER_PATH="$(command -v llvm-symbolizer-18)"
  fi
  export CC=clang
  export CXX=clang++
  export RUSTFLAGS="-Zsanitizer=address -Cdebuginfo=1"
  export ASAN_OPTIONS="${ASAN_OPTIONS:-detect_leaks=1:abort_on_error=1:symbolize=1}"
  ADDON_SANITIZE=(address)
else
  export CC=clang
  export CXX=clang++
  export RASTER_SANITIZE=undefined
  export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang++
  # Rust passes -nodefaultlibs; clang++ as linker will not pull libstdc++ unless
  # asked. UBSan C++ runtime needs RTTI / __dynamic_cast from the C++ ABI.
  export RUSTFLAGS="\
-Clink-arg=-fsanitize=undefined \
-Clink-arg=-lstdc++ \
-Cdebuginfo=1"
  ADDON_SANITIZE=(undefined)
fi

set_addon_sanitizer_flags() {
  local flags=()
  for flag in "${ADDON_SANITIZE[@]}"; do
    flags+=("-fsanitize=${flag}")
  done
  flags+=("-fno-omit-frame-pointer" "-g" "-fno-sanitize-recover=all")
  export CFLAGS="${flags[*]}"
  export CXXFLAGS="${CFLAGS}"
  export LDFLAGS="${flags[*]}"
}

clear_addon_sanitizer_flags() {
  unset CFLAGS
  unset CXXFLAGS
  unset LDFLAGS
}

assert_asan_binary() {
  local bin="$1"
  if readelf -Ws "$bin" 2>/dev/null | grep '__asan' >/dev/null; then
    return 0
  fi
  if nm -an "$bin" 2>/dev/null | grep '__asan' >/dev/null; then
    return 0
  fi
  echo "binary is not ASan-instrumented: ${bin}" >&2
  exit 1
}

assert_ubsan_binary() {
  local bin="$1"
  if readelf -Ws "$bin" 2>/dev/null | grep 'ubsan' >/dev/null; then
    return 0
  fi
  if nm -an "$bin" 2>/dev/null | grep 'ubsan' >/dev/null; then
    return 0
  fi
  echo "binary is not UBSan-instrumented: ${bin}" >&2
  exit 1
}

assert_addon_sanitized() {
  local bin="$1"
  if [[ "${SANITIZER}" == "address" ]]; then
    assert_asan_binary "${bin}"
  else
    assert_ubsan_binary "${bin}"
  fi
}

build_v8_hello_addon() {
  set_addon_sanitizer_flags
  (cd compat/v8-hello && npm run build --silent)
  clear_addon_sanitizer_flags
  assert_addon_sanitized "${V8_HELLO_ADDON}"
}

build_napi_hello_addons() {
  set_addon_sanitizer_flags
  (cd compat/napi-hello && yarn build --silent)
  clear_addon_sanitizer_flags
  assert_addon_sanitized "${NAPI_HELLO_ADDON}"
}

build_better_sqlite3_addon() {
  set_addon_sanitizer_flags
  (cd compat/better-sqlite3 && yarn install --frozen-lockfile --silent)
  (cd compat/better-sqlite3/node_modules/better-sqlite3 && npm rebuild --build-from-source --silent)
  clear_addon_sanitizer_flags
  assert_addon_sanitized "${BETTER_SQLITE3_ADDON}"
}

run_compat() {
  COMPAT_SKIP_BUILD=1 COMPAT_SKIP_NODE_BASELINE=1 node compat/run.mjs "$@"
}

run_v8_hello_compat() {
  assert_addon_sanitized "${V8_HELLO_ADDON}"
  run_compat v8-hello "${RASTER_RUNTIME}"
  assert_addon_sanitized "${V8_HELLO_ADDON}"
}

run_napi_compat() {
  assert_addon_sanitized "${NAPI_HELLO_ADDON}"
  run_compat napi-hello "${RASTER_RUNTIME}"
  assert_addon_sanitized "${NAPI_HELLO_ADDON}"
}

run_better_sqlite3_compat() {
  assert_addon_sanitized "${BETTER_SQLITE3_ADDON}"
  run_compat better-sqlite3 "${RASTER_RUNTIME}"
  assert_addon_sanitized "${BETTER_SQLITE3_ADDON}"
}

if [[ "${SANITIZER}" == "address" ]]; then
  set_addon_sanitizer_flags
  echo "[sanitizer:address] rustc unit tests (v8_compat)"
  "${CARGO[@]}" test -p v8_compat --lib --target "${TARGET}"

  echo "[sanitizer:address] rustc unit tests (raster_runtime_napi)"
  "${CARGO[@]}" test -p raster_runtime_napi --lib --target "${TARGET}"
  clear_addon_sanitizer_flags
else
  echo "[sanitizer:undefined] rustc unit tests (v8_compat shim only)"
  "${CARGO[@]}" test -p v8_compat --lib --target "${TARGET}"

  echo "[sanitizer:undefined] rustc unit tests (raster_runtime_napi)"
  "${CARGO[@]}" test -p raster_runtime_napi --lib --target "${TARGET}"
fi

echo "[sanitizer:${SANITIZER}] build raster_runtime (v8-compat)"
make js JS_MINIFY=0
if [[ "${SANITIZER}" == "address" ]]; then
  set_addon_sanitizer_flags
  "${CARGO[@]}" build -p raster_runtime --features v8-compat --target "${TARGET}"
  clear_addon_sanitizer_flags
else
  "${CARGO[@]}" build -p raster_runtime --features v8-compat --target "${TARGET}"
fi

RASTER_RUNTIME="${CARGO_TARGET_DIR}/${TARGET}/debug/raster_runtime"
export RASTER_RUNTIME

if [[ ! -x "${RASTER_RUNTIME}" ]]; then
  echo "missing raster_runtime binary: ${RASTER_RUNTIME}" >&2
  exit 1
fi

if [[ "${SANITIZER}" == "address" ]]; then
  assert_asan_binary "${RASTER_RUNTIME}"
else
  assert_ubsan_binary "${RASTER_RUNTIME}"
fi

echo "[sanitizer:${SANITIZER}] build instrumented addons"
build_v8_hello_addon
build_napi_hello_addons
build_better_sqlite3_addon

echo "[sanitizer:${SANITIZER}] compat v8-hello (5x shutdown stress)"
for _ in $(seq 1 5); do
  run_v8_hello_compat
done

echo "[sanitizer:${SANITIZER}] compat napi-hello"
run_napi_compat

echo "[sanitizer:${SANITIZER}] compat better-sqlite3"
run_better_sqlite3_compat

echo "[sanitizer:${SANITIZER}] all checks passed"
