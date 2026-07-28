#include "internal.h"

#include <v8-internal.h>
#include <v8-local-handle.h>

namespace v8 {

EscapableHandleScopeBase::EscapableHandleScopeBase(Isolate* isolate) {
  escape_slot_ = nullptr;
  auto* ctx = raster_v8_current_context();
  if (ctx) {
    // Reserve the escape slot in the parent scope arena before inner allocations begin.
    auto* reserved = raster_v8::alloc_handle_slot(ctx);
    reserved->object =
        raster_v8::shim::ObjectLayout(const_cast<raster_v8::shim::Map*>(&raster_v8::shim::Map::object_map()), 0);
    reserved->object.tagged_map = raster_v8::shim::TaggedPointer(&reserved->object);
    reserved->tagged_value = raster_v8::shim::TaggedPointer(&reserved->object);
    reserved->owns_root = true;
    escape_slot_ = reinterpret_cast<internal::Address*>(&reserved->object);
  }
  Initialize(isolate);
}

internal::Address* EscapableHandleScopeBase::EscapeSlot(internal::Address* escape_value) {
  if (!escape_value) {
    return escape_value;
  }
  if (!escape_slot_) {
    return nullptr;
  }
  const RasterV8BridgeV1* b = raster_v8_bridge();
  auto* ctx = raster_v8_current_context();
  if (!b || !ctx) {
    return escape_value;
  }
  auto* src = reinterpret_cast<raster_v8::shim::ObjectLayout*>(escape_value);
  uint64_t root_id = src->contents.root_id;
  if (root_id == 0) {
    return escape_value;
  }
  uint64_t dup = 0;
  if (b->root_dup(root_id, &dup) != RASTER_V8_OK) {
    return escape_value;
  }
  auto* dst = reinterpret_cast<raster_v8::shim::ObjectLayout*>(escape_slot_);
  *dst = *src;
  dst->contents.root_id = dup;
  dst->tagged_map = raster_v8::shim::TaggedPointer(dst);
  raster_v8::note_materialized_layout(dst);
  raster_v8::register_handle_repr(ctx, reinterpret_cast<uintptr_t>(dst), dup);
  raster_v8::register_handle_repr(ctx, static_cast<uintptr_t>(dst->tagged_map.value), dup);
  internal::Address* result = escape_slot_;
  escape_slot_ = nullptr;
  return result;
}

}  // namespace v8
