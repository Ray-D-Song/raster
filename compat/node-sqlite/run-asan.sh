#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

TARGET="${TARGET:-x86_64-unknown-linux-gnu}"
export CARGO_TARGET_DIR="$ROOT/target-sqlite-asan"
export RUSTFLAGS="-Zsanitizer=address -Cdebuginfo=1"
export CFLAGS="-fsanitize=address -fno-omit-frame-pointer -g"
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

COMPAT_SKIP_NODE_BASELINE=1 \
COMPAT_SQLITE_LIFECYCLE_LOOPS="${COMPAT_SQLITE_LIFECYCLE_LOOPS:-20}" \
COMPAT_SQLITE_BACKUP_LOOPS="${COMPAT_SQLITE_BACKUP_LOOPS:-5}" \
node compat/run.mjs node-sqlite "$RASTER_RUNTIME"

"$RASTER_RUNTIME" test -d bundle/js/__tests__/unit --filter sqlite
