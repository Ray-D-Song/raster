#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

TARGET="${TARGET:-x86_64-unknown-linux-gnu}"

if [[ "$TARGET" != "x86_64-unknown-linux-gnu" ]]; then
  echo "sqlite ASan currently supports x86_64-unknown-linux-gnu only" >&2
  exit 1
fi

unset RUSTFLAGS
unset CFLAGS
unset CXXFLAGS

export CARGO_TARGET_DIR="$ROOT/target-sqlite-asan"
export RUSTFLAGS="-Zsanitizer=address -Cdebuginfo=1"
export CC=clang
export CXX=clang++
export CFLAGS="-fsanitize=address -fno-omit-frame-pointer -g"
export CXXFLAGS="$CFLAGS"
export ASAN_OPTIONS="${ASAN_OPTIONS:-detect_leaks=1:abort_on_error=1}"

if ! rustup toolchain list | grep -q nightly; then
  rustup toolchain install nightly
fi

make js JS_MINIFY=0
cargo +nightly build \
  -p raster_runtime \
  --target "$TARGET"
node compat/node-sqlite/build-extension.mjs

RASTER_RUNTIME="$CARGO_TARGET_DIR/$TARGET/debug/raster_runtime"
if ! readelf -Ws "$RASTER_RUNTIME" 2>/dev/null | grep -q '__asan'; then
  echo "binary is not ASan-instrumented: $RASTER_RUNTIME" >&2
  exit 1
fi

SANITIZER_FAIL_RE='ERROR: AddressSanitizer|LeakSanitizer|runtime error:|SIGABRT|assert'

run_and_scan() {
  local label="$1"
  shift
  local output
  output="$("$@" 2>&1)" || {
    echo "$output"
    echo "[$label] command failed" >&2
    exit 1
  }
  echo "$output"
  if echo "$output" | grep -Eiq "$SANITIZER_FAIL_RE"; then
    echo "[$label] sanitizer or abort output detected" >&2
    exit 1
  fi
}

run_and_scan "node-sqlite parity" \
  env COMPAT_SKIP_NODE_BASELINE=1 \
  COMPAT_SQLITE_LIFECYCLE_LOOPS="${COMPAT_SQLITE_LIFECYCLE_LOOPS:-20}" \
  COMPAT_SQLITE_BACKUP_LOOPS="${COMPAT_SQLITE_BACKUP_LOOPS:-5}" \
  node compat/run.mjs node-sqlite "$RASTER_RUNTIME"

run_and_scan "sqlite unit tests" \
  "$RASTER_RUNTIME" test -d bundle/js/__tests__/unit --filter sqlite
