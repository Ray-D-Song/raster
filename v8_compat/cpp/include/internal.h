#pragma once

#include "abi_137_generated.h"
#include "raster_v8_bridge.h"
#include "shim_types.h"
#include "v8_abi_layout.h"

#include <node.h>
#include <v8-internal.h>
#include <vector>

namespace raster_v8 {

struct CallbackHandleFrame {
  std::vector<shim::ObjectLayout> layouts;
  std::vector<v8::internal::Address> values;
  std::vector<uint64_t> roots;
};

inline thread_local CallbackHandleFrame g_callback_handle_frame;
inline thread_local std::vector<CallbackHandleFrame> g_callback_handle_stack;
inline thread_local shim::ObjectLayout* g_last_materialized_layout = nullptr;

inline CallbackHandleFrame& current_callback_frame() {
  if (g_callback_handle_stack.empty()) {
    return g_callback_handle_frame;
  }
  return g_callback_handle_stack.back();
}

inline void note_materialized_layout(shim::ObjectLayout* layout) {
  g_last_materialized_layout = layout;
}

struct HandleScopeData {
  uintptr_t* next;
  uintptr_t* limit;
  int level;
  int sealed_level;

  void initialize() {
    next = nullptr;
    limit = nullptr;
    level = 0;
    sealed_level = 0;
  }
};

static_assert(sizeof(HandleScopeData) == abi137::kHandleScopeDataSize);

struct OddballValue {
  shim::ObjectLayout layout;
};

// Layout-compatible fake v8::Isolate for Node 24 / ABI 137.
struct IsolateImpl {
  void* hook0 = nullptr;
  void* hook1 = nullptr;
  uint8_t padding_before_hsd[abi137::kIsolateHandleScopeDataOffset - 16] {};
  HandleScopeData handle_scope_data {};
  uint8_t padding_after_hsd[abi137::kIsolateRootsOffset - abi137::kIsolateHandleScopeDataOffset -
                            abi137::kHandleScopeDataSize] {};
  uintptr_t roots[abi137::kRootSlotCount] {};
  OddballValue undefined_value;
  OddballValue the_hole_value;
  OddballValue null_value;
  OddballValue true_value;
  OddballValue false_value;
  OddballValue empty_string;
};

static_assert(offsetof(IsolateImpl, handle_scope_data) == abi137::kIsolateHandleScopeDataOffset);
static_assert(offsetof(IsolateImpl, roots) == abi137::kIsolateRootsOffset);

struct HandleArena {
  static constexpr size_t kBlockSize = 256;
  std::vector<std::vector<shim::HandleSlot>> blocks;
  size_t watermark = 0;
};

struct HandleScopeFrame {
  size_t watermark = 0;
};

struct ContextImpl {
  HandleArena arena;
  std::vector<HandleScopeFrame> scopes;
  uint64_t context_root_id = 0;
  uint64_t oddball_roots[abi137::kRootSlotCount] {};
};

std::vector<node::node_module*>& pending_v8_modules();
void push_pending_v8_module(node::node_module* module);
void clear_pending_v8_modules();
size_t pending_v8_modules_count();
node::node_module* take_pending_v8_module(size_t index);

ContextImpl* ctx_impl(RasterV8ContextState* ctx);
IsolateImpl* iso_impl(RasterV8IsolateState* isolate);

shim::HandleSlot* alloc_handle_slot(RasterV8ContextState* ctx);
void rewind_handle_arena(RasterV8ContextState* ctx, size_t watermark);

uintptr_t local_from_root(RasterV8ContextState* ctx, uint64_t root_id, const shim::Map* map);
uint64_t root_from_local(uintptr_t tagged);

void register_handle_repr(uintptr_t repr, uint64_t root_id);
uint64_t resolve_root_from_repr(RasterV8ContextState* ctx, uintptr_t repr);

void init_isolate_roots(IsolateImpl* isolate);
int oddball_root_index(IsolateImpl* isolate, const shim::ObjectLayout* layout);
HandleScopeData* handle_scope_data(IsolateImpl* isolate);

RasterV8Status dispatch_v8_callback(uint32_t function_id,
                                    uint64_t receiver_root,
                                    uint64_t new_target_root,
                                    const uint64_t* arg_roots,
                                    int argc,
                                    uint64_t* out_result_root);

// Set for the duration of a V8 callback dispatch; used by GetFunctionTemplateData.
extern thread_local uint32_t g_active_callback_template_id;

}  // namespace raster_v8
