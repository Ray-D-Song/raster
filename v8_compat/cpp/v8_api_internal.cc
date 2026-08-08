#include "v8_bridge_helpers.h"

#include <unordered_map>

#include <v8-internal.h>
#include <v8-persistent-handle.h>
#include <v8-weak-callback-info.h>

namespace {

bool is_callback_frame_layout(const raster_v8::shim::ObjectLayout* candidate) {
  if (candidate == nullptr ||
      (reinterpret_cast<uintptr_t>(candidate) % alignof(raster_v8::shim::ObjectLayout)) != 0) {
    return false;
  }
  auto in_frame = [&](const raster_v8::CallbackHandleFrame& frame) {
    for (const auto& layout : frame.layouts) {
      if (&layout == candidate) {
        return true;
      }
    }
    return false;
  };
  for (auto it = raster_v8::g_callback_handle_stack.rbegin();
       it != raster_v8::g_callback_handle_stack.rend(); ++it) {
    if (in_frame(*it)) {
      return true;
    }
  }
  return in_frame(raster_v8::g_callback_handle_frame);
}

bool is_live_shim_layout(const raster_v8::shim::ObjectLayout* candidate) {
  if (candidate == nullptr ||
      (reinterpret_cast<uintptr_t>(candidate) % alignof(raster_v8::shim::ObjectLayout)) != 0) {
    return false;
  }
  auto in_frame = [&](const raster_v8::CallbackHandleFrame& frame) {
    for (const auto& layout : frame.layouts) {
      if (&layout == candidate) {
        return true;
      }
    }
    return false;
  };
  for (auto it = raster_v8::g_callback_handle_stack.rbegin();
       it != raster_v8::g_callback_handle_stack.rend(); ++it) {
    if (in_frame(*it)) {
      return true;
    }
  }
  if (in_frame(raster_v8::g_callback_handle_frame)) {
    return true;
  }
  if (auto* ctx = raster_v8::bridge_ctx()) {
    for (const auto& block : raster_v8::ctx_impl(ctx)->arena.blocks) {
      for (const auto& slot : block) {
        if (&slot.object == candidate) {
          return true;
        }
      }
    }
  }
  return false;
}

raster_v8::shim::ObjectLayout* layout_for_globalize_address(uintptr_t address) {
  using raster_v8::shim::ObjectLayout;
  using raster_v8::shim::TaggedPointer;

  // FunctionCallbackInfo receivers and arguments are strong-tagged pointers into
  // the live dispatch frame and never pass through note_materialized_layout, so a
  // verifiable address always beats the last-materialized fallback.
  if (address != 0 &&
      (address & 0b11) == static_cast<uintptr_t>(TaggedPointer::Tag::StrongPointer)) {
    auto* candidate = TaggedPointer::fromRaw(address).getPtr<ObjectLayout>();
    if (is_live_shim_layout(candidate) && candidate->contents.root_id != 0) {
      return candidate;
    }
  }
  if (raster_v8::g_last_materialized_layout != nullptr) {
    return raster_v8::g_last_materialized_layout;
  }
  if (address == 0) {
    return nullptr;
  }
  if ((address & 0b11) == 0) {
    auto* direct = reinterpret_cast<ObjectLayout*>(address);
    if (direct->tagged_map.tag() == TaggedPointer::Tag::StrongPointer) {
      return direct;
    }
  }
  return nullptr;
}

raster_v8::IsolateImpl* current_iso_impl() {
  auto* isolate = raster_v8_current_isolate();
  if (!isolate) {
    return nullptr;
  }
  return raster_v8::iso_impl(isolate);
}

}  // namespace

namespace v8 {
namespace api_internal {

uintptr_t* GlobalizeReference(internal::Isolate* i_isolate, uintptr_t address) {
  (void)i_isolate;
  const RasterV8BridgeV1* b = raster_v8_bridge();
  auto* isolate = current_iso_impl();
  if (!b || !isolate) {
    return nullptr;
  }
  auto* layout = layout_for_globalize_address(address);
  if (!layout) {
    return nullptr;
  }
  uint64_t root_id = layout->contents.root_id;
  if (root_id == 0) {
    return nullptr;
  }
  // ObjectWrap::Wrap(info.This()) globalizes a borrowed receiver root that the
  // trampoline still owns until the native constructor returns; only frame layouts
  // need a dup. Normal local_from_root handles adopt the caller's root.
  if (is_callback_frame_layout(layout) && b->root_dup) {
    uint64_t owned = 0;
    if (b->root_dup(root_id, &owned) == RASTER_V8_OK && owned != 0) {
      root_id = owned;
    }
  }
  auto* persistent = new raster_v8::shim::ObjectLayout(
      const_cast<raster_v8::shim::Map*>(&raster_v8::shim::Map::object_map()), root_id);
  persistent->function_id = layout->function_id;
  if (persistent->function_id != 0) {
    isolate->layout_to_function_id[reinterpret_cast<uintptr_t>(persistent)] = persistent->function_id;
  }
  auto* cell = new uintptr_t(reinterpret_cast<uintptr_t>(persistent));
  uintptr_t ctx_key = reinterpret_cast<uintptr_t>(raster_v8::bridge_ctx());
  if (auto* ctx = raster_v8::bridge_ctx()) {
    ctx_key = raster_v8::ctx_impl(ctx)->quickjs_context_key;
  }
  isolate->persistents[cell] = raster_v8::PersistentSlot{root_id, false, 0, ctx_key};
  raster_v8::g_last_materialized_layout = nullptr;
  return cell;
}

void DisposeGlobal(uintptr_t* location) {
  if (!location) {
    return;
  }
  const RasterV8BridgeV1* b = raster_v8_bridge();
  auto* isolate = current_iso_impl();
  if (!isolate) {
    return;
  }
  auto it = isolate->persistents.find(location);
  if (it != isolate->persistents.end()) {
    if (b && b->root_drop && it->second.root_id != 0) {
      b->root_drop(it->second.root_id);
    }
    auto* layout = reinterpret_cast<raster_v8::shim::ObjectLayout*>(*location);
    const auto layout_addr = reinterpret_cast<uintptr_t>(layout);
    isolate->layout_to_root.erase(layout_addr);
    isolate->layout_to_function_id.erase(layout_addr);
    delete layout;
    isolate->persistents.erase(it);
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
  auto* isolate = current_iso_impl();
  if (!isolate) {
    return;
  }
  auto it = isolate->persistents.find(location);
  if (it == isolate->persistents.end() || !b) {
    return;
  }
  void* object_ptr = nullptr;
  if (it->second.root_id != 0) {
    raster_v8_object_ptr_for_root(raster_v8::bridge_ctx(), it->second.root_id, &object_ptr);
  }
  if (b->register_weak_callback) {
    b->register_weak_callback(
        raster_v8::bridge_ctx(),
        it->second.root_id,
        parameter,
        reinterpret_cast<void*>(callback));
  }
  if (b->root_make_weak) {
    b->root_make_weak(it->second.root_id);
  } else if (b->root_drop) {
    b->root_drop(it->second.root_id);
  }
  it->second.weak_object_ptr = reinterpret_cast<uintptr_t>(object_ptr);
  it->second.is_weak = true;
  it->second.root_id = 0;
}

void* ClearWeak(internal::Address* location) {
  if (!location) {
    return nullptr;
  }
  auto* slot = reinterpret_cast<uintptr_t*>(location);
  auto* isolate = current_iso_impl();
  if (!isolate) {
    return nullptr;
  }
  auto it = isolate->persistents.find(slot);
  if (it == isolate->persistents.end()) {
    return nullptr;
  }
  if (it->second.is_weak) {
    if (it->second.weak_object_ptr != 0) {
      void* object_ptr = reinterpret_cast<void*>(it->second.weak_object_ptr);
      raster_v8_unregister_weak_for_object_ptr(raster_v8::bridge_ctx(), object_ptr);
      uint64_t new_root = 0;
      if (raster_v8_root_restrong_from_object_ptr(raster_v8::bridge_ctx(), object_ptr, &new_root) ==
          RASTER_V8_OK) {
        it->second.root_id = new_root;
        auto* layout = reinterpret_cast<raster_v8::shim::ObjectLayout*>(*slot);
        layout->contents.root_id = new_root;
      }
      it->second.weak_object_ptr = 0;
    }
    it->second.is_weak = false;
  }
  return nullptr;
}

}  // namespace api_internal
}  // namespace v8

extern "C" void raster_v8_persistent_counts_for_context(void* isolate,
                                                        uintptr_t context_key,
                                                        size_t* strong_out,
                                                        size_t* weak_out) {
  raster_v8::persistent_counts_for_context(raster_v8::iso_impl(
                                               reinterpret_cast<RasterV8IsolateState*>(isolate)),
                                           context_key,
                                           strong_out,
                                           weak_out);
}

extern "C" size_t raster_v8_dispose_strong_context_persistents(void* isolate,
                                                               uintptr_t context_key) {
  return raster_v8::dispose_strong_context_persistents(
      raster_v8::iso_impl(reinterpret_cast<RasterV8IsolateState*>(isolate)), context_key);
}

extern "C" size_t raster_v8_dispose_weak_context_persistents(void* isolate,
                                                             uintptr_t context_key) {
  return raster_v8::dispose_weak_context_persistents(
      raster_v8::iso_impl(reinterpret_cast<RasterV8IsolateState*>(isolate)), context_key);
}

extern "C" void raster_v8_dispose_context_persistents(void* isolate, uintptr_t context_key) {
  raster_v8::dispose_context_persistents(raster_v8::iso_impl(reinterpret_cast<RasterV8IsolateState*>(isolate)),
                                         context_key);
}

extern "C" void raster_v8_dispose_all_persistents(void) {
  auto* isolate = current_iso_impl();
  if (!isolate) {
    return;
  }
  raster_v8::dispose_isolate_persistents(isolate);
}

extern "C" RasterV8Status raster_v8_root_id_for_persistent_layout(void* layout,
                                                                 uint64_t* out_root_id) {
  if (!layout || !out_root_id) {
    return RASTER_V8_ERROR;
  }
  auto* isolate = current_iso_impl();
  if (!isolate) {
    return RASTER_V8_ERROR;
  }
  auto addr = reinterpret_cast<uintptr_t>(layout);
  if (auto it = isolate->layout_to_root.find(addr); it != isolate->layout_to_root.end()) {
    *out_root_id = it->second;
    return RASTER_V8_OK;
  }
  auto* object_layout = reinterpret_cast<raster_v8::shim::ObjectLayout*>(layout);
  for (const auto& [cell, slot] : isolate->persistents) {
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
  auto* isolate = current_iso_impl();
  if (!isolate) {
    return;
  }
  isolate->layout_to_root[reinterpret_cast<uintptr_t>(layout)] = root_id;
}

extern "C" void raster_v8_register_layout_function_id(void* layout, uint32_t function_id) {
  if (!layout || function_id == 0) {
    return;
  }
  auto* isolate = current_iso_impl();
  if (!isolate) {
    return;
  }
  isolate->layout_to_function_id[reinterpret_cast<uintptr_t>(layout)] = function_id;
}

extern "C" RasterV8Status raster_v8_function_id_for_layout(void* layout, uint32_t* out_function_id) {
  if (!layout || !out_function_id) {
    return RASTER_V8_ERROR;
  }
  auto* isolate = current_iso_impl();
  if (!isolate) {
    return RASTER_V8_ERROR;
  }
  auto it = isolate->layout_to_function_id.find(reinterpret_cast<uintptr_t>(layout));
  if (it == isolate->layout_to_function_id.end()) {
    return RASTER_V8_ERROR;
  }
  *out_function_id = it->second;
  return RASTER_V8_OK;
}
