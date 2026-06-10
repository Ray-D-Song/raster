#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ANDROID_TEMPLATE_DIR="$ROOT_DIR/platforms/android/template"
ANDROID_BUILD_DIR="$ROOT_DIR/target/raster/android"
PROFILE="debug"
GRADLE_TASK="assembleDebug"
CARGO_PROFILE_ARGS=()

if [[ "${1:-}" == "--release" ]]; then
  PROFILE="release"
  GRADLE_TASK="assembleRelease"
  CARGO_PROFILE_ARGS=(--release)
fi

if ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "cargo-ndk is required. Install it with: cargo install cargo-ndk" >&2
  exit 1
fi

cd "$ROOT_DIR"

export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$ANDROID_HOME}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort -V | tail -n 1)}"
export ANDROID_NDK="${ANDROID_NDK:-$ANDROID_NDK_HOME}"

if [[ -z "$ANDROID_NDK_HOME" || ! -d "$ANDROID_NDK_HOME" ]]; then
  echo "Android NDK not found. Set ANDROID_NDK_HOME or install an NDK under $ANDROID_HOME/ndk." >&2
  exit 1
fi

if command -v ninja >/dev/null 2>&1; then
  export CMAKE_GENERATOR="${CMAKE_GENERATOR:-Ninja}"
  export CMAKE_MAKE_PROGRAM="${CMAKE_MAKE_PROGRAM:-$(command -v ninja)}"
fi

NDK_TOOLCHAIN_BIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin"
export CMAKE_C_COMPILER="${CMAKE_C_COMPILER:-$NDK_TOOLCHAIN_BIN/aarch64-linux-android31-clang}"
export CMAKE_CXX_COMPILER="${CMAKE_CXX_COMPILER:-$NDK_TOOLCHAIN_BIN/aarch64-linux-android31-clang++}"

pnpm run build:template

rm -rf "$ANDROID_BUILD_DIR"
mkdir -p "$(dirname "$ANDROID_BUILD_DIR")"
cp -R "$ANDROID_TEMPLATE_DIR" "$ANDROID_BUILD_DIR"

mkdir -p "$ANDROID_BUILD_DIR/app/src/main/assets/raster"
cp target/raster/template/app.js "$ANDROID_BUILD_DIR/app/src/main/assets/raster/app.js"
if [[ -f target/raster/template/app.js.map ]]; then
  cp target/raster/template/app.js.map "$ANDROID_BUILD_DIR/app/src/main/assets/raster/app.js.map"
fi

rustup target add aarch64-linux-android >/dev/null
cargo ndk \
  -t arm64-v8a \
  -P 31 \
  -o "$ANDROID_BUILD_DIR/app/src/main/jniLibs" \
  build --lib ${CARGO_PROFILE_ARGS+"${CARGO_PROFILE_ARGS[@]}"}

cd "$ANDROID_BUILD_DIR"
if [[ -x ./gradlew ]]; then
  ./gradlew "$GRADLE_TASK"
else
  gradle "$GRADLE_TASK"
fi

echo "Built Android $PROFILE APK under target/raster/android/app/build/outputs/apk/"
