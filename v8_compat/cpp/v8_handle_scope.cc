#include "internal.h"

#include <utility>

#include <v8-internal.h>
#include <v8-local-handle.h>

namespace raster_v8 {

static HandleScopeData* isolate_handle_scope_data(v8::Isolate* isolate) {
  return handle_scope_data(reinterpret_cast<IsolateImpl*>(isolate));
}

}  // namespace raster_v8

namespace v8 {

void HandleScope::Initialize(Isolate* isolate) {
  auto* data = raster_v8::isolate_handle_scope_data(isolate);
  i_isolate_ = reinterpret_cast<internal::Isolate*>(isolate);
  prev_next_ = reinterpret_cast<internal::Address*>(data->next);
  prev_limit_ = reinterpret_cast<internal::Address*>(data->limit);
  data->level++;
  if (auto* ctx = raster_v8_current_context()) {
    auto* impl = raster_v8::ctx_impl(ctx);
    impl->scopes.push_back(raster_v8::HandleScopeFrame{impl->arena.watermark});
  }
}

HandleScope::HandleScope(Isolate* isolate) {
  Initialize(isolate);
}

HandleScope::~HandleScope() {
  if (!i_isolate_) {
    return;
  }
  auto* isolate = reinterpret_cast<Isolate*>(i_isolate_);
  auto* data = raster_v8::isolate_handle_scope_data(isolate);
  std::swap(data->next, reinterpret_cast<uintptr_t*&>(prev_next_));
  data->level--;
  if (reinterpret_cast<uintptr_t*>(data->limit) != reinterpret_cast<uintptr_t*>(prev_limit_)) {
    data->limit = reinterpret_cast<uintptr_t*>(prev_limit_);
  }
  if (auto* ctx = raster_v8_current_context()) {
    auto* impl = raster_v8::ctx_impl(ctx);
    if (!impl->scopes.empty()) {
      raster_v8::rewind_handle_arena(ctx, impl->scopes.back().watermark);
      impl->scopes.pop_back();
    }
  }
}

internal::Address* HandleScope::CreateHandle(internal::Isolate* i_isolate,
                                             internal::Address value) {
  (void)i_isolate;
  auto* ctx = raster_v8_current_context();
  if (!ctx) {
    return nullptr;
  }
  auto* src = reinterpret_cast<raster_v8::shim::ObjectLayout*>(value);
  auto* slot = raster_v8::alloc_handle_slot(ctx);
  slot->object = *src;
  slot->object.tagged_map = raster_v8::shim::TaggedPointer(&slot->object);
  slot->tagged_value = raster_v8::shim::TaggedPointer(&slot->object);
  raster_v8::note_materialized_layout(&slot->object);
  raster_v8::register_handle_repr(ctx, reinterpret_cast<uintptr_t>(&slot->object),
                                  slot->object.contents.root_id);
  raster_v8::register_handle_repr(ctx, static_cast<uintptr_t>(slot->object.tagged_map.value),
                                  slot->object.contents.root_id);
  // Indirect Local::ptr() reads the first word of the in-place object layout.
  return reinterpret_cast<internal::Address*>(&slot->object);
}

}  // namespace v8
