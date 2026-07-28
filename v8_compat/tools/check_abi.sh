#!/usr/bin/env bash
# Verify V8/Node ABI 137 constants against Node 24.3.0 headers (nvm).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TOOLS="$ROOT/v8_compat/tools"
GEN_HDR="$ROOT/v8_compat/cpp/include/abi_137_generated.h"

node_include() {
  if [[ -n "${RASTER_NODE24_INCLUDE:-}" ]]; then
    echo "$RASTER_NODE24_INCLUDE"
    return
  fi
  if [[ -n "${HOME:-}" ]]; then
    local nvm="$HOME/.nvm/versions/node/v24.3.0/include/node"
    if [[ -f "$nvm/v8.h" ]]; then
      echo "$nvm"
      return
    fi
  fi
  echo "Node 24.3.0 headers not found. Install via nvm or set RASTER_NODE24_INCLUDE." >&2
  exit 1
}

NODE_INC="$(node_include)"
CXX="${CXX:-clang++}"

build_probe() {
  local src="$1"
  local out="$2"
  shift 2
  "$CXX" -std=c++20 -DNODE_MODULE_VERSION=137 \
    -I"$NODE_INC" \
    -I"$ROOT/v8_compat/cpp/include" \
    "$src" -o "$out" "$@"
}

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

expect_layout() {
  local name="$1"
  local want="$2"
  local got
  got="$(echo "$LAYOUT_OUT" | sed -n "s/^${name}=//p")"
  if [[ "$got" != "$want" ]]; then
    echo "ABI drift: $name expected $want, got $got" >&2
    exit 1
  fi
}

build_probe "$TOOLS/layout_probe.cc" "$TMP/layout_probe"
LAYOUT_OUT="$("$TMP/layout_probe")"

expect_layout kIsolateHandleScopeDataOffset 560
expect_layout kIsolateRootsOffset 640
expect_layout kHandleScopeDataSize 24

HANDLE_SCOPE_SIZE="$(echo "$LAYOUT_OUT" | sed -n 's/^sizeof(HandleScope)=//p')"
ESCAPABLE_SIZE="$(echo "$LAYOUT_OUT" | sed -n 's/^sizeof(EscapableHandleScope)=//p')"
if [[ -z "$HANDLE_SCOPE_SIZE" || -z "$ESCAPABLE_SIZE" || "$ESCAPABLE_SIZE" -lt "$HANDLE_SCOPE_SIZE" ]]; then
  echo "ABI drift: EscapableHandleScope=$ESCAPABLE_SIZE HandleScope=$HANDLE_SCOPE_SIZE" >&2
  exit 1
fi

build_probe "$TOOLS/abi-generator.cc" "$TMP/abi-generator"
ABI_JSON="$("$TMP/abi-generator")"

expect_macro() {
  local macro="$1"
  local want="$2"
  local got
  got="$(grep "#define $macro " "$GEN_HDR" | awk '{print $3}')"
  if [[ "$got" != "$want" ]]; then
    echo "ABI drift: $macro in abi_137_generated.h is $got, Node 24.3.0 has $want" >&2
    exit 1
  fi
}

json_int() {
  local key="$1"
  echo "$ABI_JSON" | sed -n "s/.*\"$key\": \\([0-9-]*\\).*/\\1/p" | head -1
}

expect_macro RASTER_V8_K_UNDEFINED_VALUE_ROOT_INDEX "$(json_int kUndefinedValueRootIndex)"
expect_macro RASTER_V8_K_THE_HOLE_VALUE_ROOT_INDEX "$(json_int kTheHoleValueRootIndex)"
expect_macro RASTER_V8_K_NULL_VALUE_ROOT_INDEX "$(json_int kNullValueRootIndex)"
expect_macro RASTER_V8_K_TRUE_VALUE_ROOT_INDEX "$(json_int kTrueValueRootIndex)"
expect_macro RASTER_V8_K_FALSE_VALUE_ROOT_INDEX "$(json_int kFalseValueRootIndex)"
expect_macro RASTER_V8_K_EMPTY_STRING_ROOT_INDEX "$(json_int kEmptyStringRootIndex)"
expect_macro RASTER_V8_FUNCTION_CALLBACK_K_RETURN_VALUE_INDEX "$(json_int function_callback_k_return_value_index)"
expect_macro RASTER_V8_PROPERTY_CALLBACK_K_RETURN_VALUE_INDEX "$(json_int property_callback_k_return_value_index)"
expect_macro RASTER_V8_PROPERTY_CALLBACK_K_THIS_INDEX "$(json_int property_callback_k_this_index)"

echo "V8 ABI 137 check OK (Node 24.3.0 headers at $NODE_INC)"
