#include "internal.h"

#include <cstdio>

namespace {

const RasterV8BridgeV1* g_bridge = nullptr;
RasterV8OddballRootFn g_oddball_root_fn = nullptr;
thread_local RasterV8ContextState* g_current_ctx = nullptr;
thread_local RasterV8IsolateState* g_current_isolate = nullptr;

}  // namespace

extern "C" void raster_v8_bind_bridge(const RasterV8BridgeV1* bridge) {
  g_bridge = bridge;
}

extern "C" void raster_v8_set_oddball_root_fn(RasterV8OddballRootFn fn) {
  g_oddball_root_fn = fn;
}

extern "C" const RasterV8BridgeV1* raster_v8_bridge(void) {
  return g_bridge;
}

extern "C" RasterV8ContextState* raster_v8_current_context(void) {
  return g_current_ctx;
}

extern "C" RasterV8IsolateState* raster_v8_current_isolate(void) {
  return g_current_isolate;
}

extern "C" void raster_v8_set_current_context(RasterV8ContextState* ctx) {
  g_current_ctx = ctx;
}

extern "C" void raster_v8_set_current_isolate(RasterV8IsolateState* isolate) {
  g_current_isolate = isolate;
}

extern "C" RasterV8IsolateState* raster_v8_create_isolate(void) {
  auto* isolate = new raster_v8::IsolateImpl();
  raster_v8::init_isolate_roots(isolate);
  return reinterpret_cast<RasterV8IsolateState*>(isolate);
}

extern "C" void raster_v8_destroy_isolate(RasterV8IsolateState* isolate) {
  if (g_current_isolate == isolate) {
    g_current_isolate = nullptr;
  }
  raster_v8::dispose_isolate_persistents(raster_v8::iso_impl(isolate));
  delete raster_v8::iso_impl(isolate);
}

extern "C" RasterV8ContextState* raster_v8_create_context(void) {
  return reinterpret_cast<RasterV8ContextState*>(new raster_v8::ContextImpl());
}

extern "C" void raster_v8_destroy_context(RasterV8ContextState* ctx) {
  if (g_current_ctx == ctx) {
    g_current_ctx = nullptr;
  }
  delete raster_v8::ctx_impl(ctx);
}

extern "C" void raster_v8_open_handle_scope(RasterV8ContextState* ctx) {
  auto* impl = raster_v8::ctx_impl(ctx);
  impl->scopes.push_back(raster_v8::HandleScopeFrame{impl->arena.watermark});
}

extern "C" void raster_v8_close_handle_scope(RasterV8ContextState* ctx) {
  auto* impl = raster_v8::ctx_impl(ctx);
  if (impl->scopes.empty()) {
    return;
  }
  raster_v8::rewind_handle_arena(ctx, impl->scopes.back().watermark);
  impl->scopes.pop_back();
}

extern "C" void raster_v8_set_context_root_id(RasterV8ContextState* ctx, uint64_t root_id) {
  raster_v8::ctx_impl(ctx)->context_root_id = root_id;
}

extern "C" void raster_v8_set_context_quickjs_key(RasterV8ContextState* ctx, uintptr_t key) {
  raster_v8::ctx_impl(ctx)->quickjs_context_key = key;
}

extern "C" uint64_t raster_v8_context_root_id(RasterV8ContextState* ctx) {
  return raster_v8::ctx_impl(ctx)->context_root_id;
}

extern "C" RasterV8Status raster_v8_oddball_root(RasterV8ContextState* ctx,
                                                 int root_index,
                                                 uint64_t* out_root_id) {
  if (!ctx || !out_root_id || root_index < 0 ||
      static_cast<size_t>(root_index) >= raster_v8::abi137::kRootSlotCount) {
    return RASTER_V8_ERROR;
  }
  auto* impl = raster_v8::ctx_impl(ctx);
  if (impl->oddball_roots[root_index] != 0) {
    *out_root_id = impl->oddball_roots[root_index];
    return RASTER_V8_OK;
  }
  if (!g_oddball_root_fn) {
    return RASTER_V8_UNSUPPORTED;
  }
  uint64_t root_id = 0;
  RasterV8Status status = g_oddball_root_fn(ctx, root_index, &root_id);
  if (status != RASTER_V8_OK) {
    return status;
  }
  impl->oddball_roots[root_index] = root_id;
  *out_root_id = root_id;
  return RASTER_V8_OK;
}

extern "C" size_t raster_v8_pending_modules_count(void) {
  return raster_v8::pending_v8_modules_count();
}

extern "C" void* raster_v8_take_pending_module(size_t index) {
  return raster_v8::take_pending_v8_module(index);
}
