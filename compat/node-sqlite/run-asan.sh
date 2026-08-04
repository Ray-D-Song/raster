#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

export RUSTFLAGS="-Zsanitizer=address"
export ASAN_OPTIONS="${ASAN_OPTIONS:-detect_leaks=1:abort_on_error=1}"

if ! rustup toolchain list | grep -q nightly; then
  rustup toolchain install nightly
fi

make js JS_MINIFY=0
cargo +nightly build -p raster_runtime
node compat/node-sqlite/build-extension.mjs

COMPAT_SKIP_NODE_BASELINE=1 \
COMPAT_SQLITE_LIFECYCLE_LOOPS="${COMPAT_SQLITE_LIFECYCLE_LOOPS:-20}" \
COMPAT_SQLITE_BACKUP_LOOPS="${COMPAT_SQLITE_BACKUP_LOOPS:-5}" \
node compat/run.mjs node-sqlite ./target/debug/raster_runtime

./target/debug/raster_runtime test -d bundle/js/__tests__/unit --filter sqlite
