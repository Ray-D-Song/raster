#include "v8_bridge_helpers.h"

#include <unordered_map>

#include <v8-internal.h>
#include <v8-persistent-handle.h>
#include <v8-weak-callback-info.h>

namespace {

struct PersistentSlot {
  uint64_t root_id = 0;
  bool is_weak = false;
};

std::unordered_map<uintptr_t*, PersistentSlot> g_persistents;
std::unordered_map<uintptr_t, uint64_t> g_layout_to_root;
std::unordered_map<uintptr_t, uint32_t> g_layout_to_function_id;

raster_v8::shim::ObjectLayout* layout_for_globalize_address(uintptr_t address) {
  if (raster_v8::g_last_materialized_layout != nullptr) {
    return raster_v8::g_last_materialized_layout;
  }
  if (address == 0) {
    return nullptr;
  }
  if ((address & 0b11) == 0) {
    auto* direct = reinterpret_cast<raster_v8::shim::ObjectLayout*>(address);
    if (direct->tagged_map.tag() == raster_v8::shim::TaggedPointer::Tag::StrongPointer) {
      return direct;
    }
  }
  return nullptr;
}

}  // namespace

namespace v8 {
namespace api_internal {

uintptr_t* GlobalizeReference(internal::Isolate* i_isolate, uintptr_t address) {
  (void)i_isolate;
  const RasterV8BridgeV1* b = raster_v8_bridge();
  if (!b || !b->root_dup) {
    return nullptr;
  }
  auto* layout = layout_for_globalize_address(address);
  if (!layout) {
    return nullptr;
  }
  uint64_t root_id = layout->contents.root_id;
  uint64_t dup = 0;
  if (b->root_dup(root_id, &dup) != RASTER_V8_OK) {
    return nullptr;
  }
  layout->contents.root_id = dup;
  g_layout_to_root[reinterpret_cast<uintptr_t>(layout)] = dup;
  auto* persistent = new raster_v8::shim::ObjectLayout(
      const_cast<raster_v8::shim::Map*>(&raster_v8::shim::Map::object_map()), dup);
  persistent->function_id = layout->function_id;
  if (persistent->function_id != 0) {
    g_layout_to_function_id[reinterpret_cast<uintptr_t>(persistent)] = persistent->function_id;
  }
  auto* cell = new uintptr_t(reinterpret_cast<uintptr_t>(persistent));
  g_persistents[cell] = PersistentSlot{dup, false};
  raster_v8::g_last_materialized_layout = nullptr;
  return cell;
}

void DisposeGlobal(uintptr_t* location) {
  if (!location) {
    return;
  }
  const RasterV8BridgeV1* b = raster_v8_bridge();
  auto it = g_persistents.find(location);
  if (it != g_persistents.end()) {
    if (b && b->root_drop && it->second.root_id != 0) {
      b->root_drop(it->second.root_id);
    }
    delete reinterpret_cast<raster_v8::shim::ObjectLayout*>(*location);
    g_persistents.erase(it);
  }
  delete location;
}

void MakeWeak(uintptr_t* location,
              void* parameter,
              WeakCallbackInfo<void>::Callback callback,
              WeakCallbackType type) {
  (void)type;
  if (!location) {
    return;
  }
  const RasterV8BridgeV1* b = raster_v8_bridge();
  auto it = g_persistents.find(location);
  if (it == g_persistents.end() || !b) {
    return;
  }
  if (b->register_weak_callback) {
    b->register_weak_callback(
        raster_v8::bridge_ctx(),
        it->second.root_id,
        parameter,
        reinterpret_cast<void (*)(const void*, int)>(callback));
  }
  if (b->root_drop) {
    b->root_drop(it->second.root_id);
  }
  it->second.is_weak = true;
  it->second.root_id = 0;
}

void* ClearWeak(internal::Address* location) {
  if (!location) {
    return nullptr;
  }
  auto* slot = reinterpret_cast<uintptr_t*>(location);
  auto it = g_persistents.find(slot);
  if (it != g_persistents.end()) {
    it->second.is_weak = false;
  }
  return nullptr;
}

}  // namespace api_internal
}  // namespace v8

extern "C" RasterV8Status raster_v8_root_id_for_persistent_layout(void* layout,
                                                                 uint64_t* out_root_id) {
  if (!layout || !out_root_id) {
    return RASTER_V8_ERROR;
  }
  auto addr = reinterpret_cast<uintptr_t>(layout);
  if (auto it = g_layout_to_root.find(addr); it != g_layout_to_root.end()) {
    *out_root_id = it->second;
    return RASTER_V8_OK;
  }
  auto* object_layout = reinterpret_cast<raster_v8::shim::ObjectLayout*>(layout);
  for (const auto& [cell, slot] : g_persistents) {
    if (slot.root_id == 0) {
      continue;
    }
    if (reinterpret_cast<raster_v8::shim::ObjectLayout*>(*cell) == object_layout ||
        object_layout->contents.root_id == slot.root_id) {
      *out_root_id = slot.root_id;
      return RASTER_V8_OK;
    }
  }
  return RASTER_V8_ERROR;
}

extern "C" void raster_v8_register_layout_root(void* layout, uint64_t root_id) {
  if (!layout || root_id == 0) {
    return;
  }
  g_layout_to_root[reinterpret_cast<uintptr_t>(layout)] = root_id;
}

extern "C" void raster_v8_register_layout_function_id(void* layout, uint32_t function_id) {
  if (!layout || function_id == 0) {
    return;
  }
  g_layout_to_function_id[reinterpret_cast<uintptr_t>(layout)] = function_id;
}

extern "C" RasterV8Status raster_v8_function_id_for_layout(void* layout, uint32_t* out_function_id) {
  if (!layout || !out_function_id) {
    return RASTER_V8_ERROR;
  }
  auto it = g_layout_to_function_id.find(reinterpret_cast<uintptr_t>(layout));
  if (it == g_layout_to_function_id.end()) {
    return RASTER_V8_ERROR;
  }
  *out_function_id = it->second;
  return RASTER_V8_OK;
}
