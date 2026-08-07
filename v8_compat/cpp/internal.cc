#include "internal.h"
#include "abi_137_generated.h"
#include "raster_v8_bridge.h"

#include <cstdio>

namespace raster_v8 {

void register_handle_repr(RasterV8ContextState* ctx, uintptr_t repr, uint64_t root_id) {
  if (!ctx || repr == 0 || root_id == 0) {
    return;
  }
  ctx_impl(ctx)->repr_to_root[repr] = root_id;
}

void unregister_handle_repr(RasterV8ContextState* ctx, uintptr_t repr) {
  if (!ctx || repr == 0) {
    return;
  }
  ctx_impl(ctx)->repr_to_root.erase(repr);
}

static uint64_t root_from_layout(RasterV8ContextState* ctx, shim::ObjectLayout* layout) {
  if (!layout) {
    return 0;
  }
  if (layout->contents.root_id != 0) {
    return layout->contents.root_id;
  }
  auto* isolate = raster_v8_current_isolate();
  if (!isolate) {
    return 0;
  }
  const int root_index = oddball_root_index(iso_impl(isolate), layout);
  if (root_index < 0) {
    return 0;
  }
  uint64_t root_id = 0;
  if (raster_v8_oddball_root(ctx, root_index, &root_id) != RASTER_V8_OK) {
    return 0;
  }
  layout->contents.root_id = root_id;
  return root_id;
}

uint64_t resolve_root_from_repr(RasterV8ContextState* ctx, uintptr_t repr) {
  if (repr == 0) {
    return 0;
  }
  auto* impl = ctx_impl(ctx);
  if (auto it = impl->repr_to_root.find(repr); it != impl->repr_to_root.end()) {
    return it->second;
  }
  for (const auto& block : impl->arena.blocks) {
    for (const auto& slot : block) {
      const uintptr_t object_addr = reinterpret_cast<uintptr_t>(&slot.object);
      if (object_addr == repr && slot.object.contents.root_id != 0) {
        register_handle_repr(ctx, repr, slot.object.contents.root_id);
        return slot.object.contents.root_id;
      }
    }
  }
  for (const auto& block : impl->arena.blocks) {
    for (const auto& slot : block) {
      const uintptr_t tagged_word = slot.object.tagged_map.value;
      if (tagged_word == repr && slot.object.contents.root_id != 0) {
        register_handle_repr(ctx, repr, slot.object.contents.root_id);
        return slot.object.contents.root_id;
      }
    }
  }
  auto* layout = reinterpret_cast<shim::ObjectLayout*>(repr);
  if ((repr & 0b11) == static_cast<uintptr_t>(shim::TaggedPointer::Tag::StrongPointer)) {
    if (auto* from_tag = shim::TaggedPointer::fromRaw(repr).getPtr<shim::ObjectLayout>()) {
      if (uint64_t root_id = root_from_layout(ctx, from_tag)) {
        return root_id;
      }
    }
  }
  if (layout->tagged_map.tag() == shim::TaggedPointer::Tag::StrongPointer) {
    if (layout->tagged_map.getPtr<shim::ObjectLayout>() == layout) {
      if (uint64_t root_id = root_from_layout(ctx, layout)) {
        return root_id;
      }
    }
    const auto* map = layout->tagged_map.getPtr<shim::Map>();
    if (map != nullptr &&
        map->instance_type == static_cast<uint8_t>(shim::InstanceType::Object)) {
      return layout->contents.root_id;
    }
    if (uint64_t root_id = root_from_layout(ctx, layout)) {
      return root_id;
    }
  }
  return 0;
}

void init_isolate_roots(IsolateImpl* isolate) {
  isolate->handle_scope_data.initialize();
  isolate->undefined_value.layout =
      shim::ObjectLayout(const_cast<shim::Map*>(&shim::Map::oddball_map()), 0);
  isolate->the_hole_value.layout =
      shim::ObjectLayout(const_cast<shim::Map*>(&shim::Map::oddball_map()), 0);
  isolate->null_value.layout =
      shim::ObjectLayout(const_cast<shim::Map*>(&shim::Map::oddball_map()), 0);
  isolate->true_value.layout =
      shim::ObjectLayout(const_cast<shim::Map*>(&shim::Map::oddball_map()), 0);
  isolate->false_value.layout =
      shim::ObjectLayout(const_cast<shim::Map*>(&shim::Map::oddball_map()), 0);
  isolate->empty_string.layout =
      shim::ObjectLayout(const_cast<shim::Map*>(&shim::Map::string_map()), 0);

  isolate->roots[RASTER_V8_K_UNDEFINED_VALUE_ROOT_INDEX] =
      reinterpret_cast<uintptr_t>(&isolate->undefined_value.layout);
  isolate->roots[RASTER_V8_K_THE_HOLE_VALUE_ROOT_INDEX] =
      reinterpret_cast<uintptr_t>(&isolate->the_hole_value.layout);
  isolate->roots[RASTER_V8_K_NULL_VALUE_ROOT_INDEX] =
      reinterpret_cast<uintptr_t>(&isolate->null_value.layout);
  isolate->roots[RASTER_V8_K_TRUE_VALUE_ROOT_INDEX] =
      reinterpret_cast<uintptr_t>(&isolate->true_value.layout);
  isolate->roots[RASTER_V8_K_FALSE_VALUE_ROOT_INDEX] =
      reinterpret_cast<uintptr_t>(&isolate->false_value.layout);
  isolate->roots[RASTER_V8_K_EMPTY_STRING_ROOT_INDEX] =
      reinterpret_cast<uintptr_t>(&isolate->empty_string.layout);
}

int oddball_root_index(IsolateImpl* isolate, const shim::ObjectLayout* layout) {
  if (layout == &isolate->undefined_value.layout) {
    return RASTER_V8_K_UNDEFINED_VALUE_ROOT_INDEX;
  }
  if (layout == &isolate->the_hole_value.layout) {
    return RASTER_V8_K_THE_HOLE_VALUE_ROOT_INDEX;
  }
  if (layout == &isolate->null_value.layout) {
    return RASTER_V8_K_NULL_VALUE_ROOT_INDEX;
  }
  if (layout == &isolate->true_value.layout) {
    return RASTER_V8_K_TRUE_VALUE_ROOT_INDEX;
  }
  if (layout == &isolate->false_value.layout) {
    return RASTER_V8_K_FALSE_VALUE_ROOT_INDEX;
  }
  if (layout == &isolate->empty_string.layout) {
    return RASTER_V8_K_EMPTY_STRING_ROOT_INDEX;
  }
  return -1;
}

HandleScopeData* handle_scope_data(IsolateImpl* isolate) {
  return &isolate->handle_scope_data;
}

ContextImpl* ctx_impl(RasterV8ContextState* ctx) {
  return reinterpret_cast<ContextImpl*>(ctx);
}

IsolateImpl* iso_impl(RasterV8IsolateState* isolate) {
  return reinterpret_cast<IsolateImpl*>(isolate);
}

std::vector<node::node_module*>& pending_v8_modules() {
  static thread_local std::vector<node::node_module*> modules;
  return modules;
}

void push_pending_v8_module(node::node_module* module) {
  pending_v8_modules().push_back(module);
}

void clear_pending_v8_modules() {
  pending_v8_modules().clear();
}

size_t pending_v8_modules_count() {
  return pending_v8_modules().size();
}

node::node_module* take_pending_v8_module(size_t index) {
  auto& modules = pending_v8_modules();
  if (index >= modules.size()) {
    return nullptr;
  }
  auto* module = modules[index];
  modules.erase(modules.begin() + static_cast<std::ptrdiff_t>(index));
  return module;
}

shim::HandleSlot* alloc_handle_slot(RasterV8ContextState* ctx) {
  auto* impl = ctx_impl(ctx);
  auto& arena = impl->arena;
  if (arena.watermark % HandleArena::kBlockSize == 0) {
    arena.blocks.emplace_back(HandleArena::kBlockSize);
  }
  size_t block = arena.watermark / HandleArena::kBlockSize;
  size_t index = arena.watermark % HandleArena::kBlockSize;
  auto* slot = &arena.blocks[block][index];
  arena.watermark++;
  return slot;
}

void rewind_handle_arena(RasterV8ContextState* ctx, size_t watermark) {
  const RasterV8BridgeV1* bridge = raster_v8_bridge();
  auto* impl = ctx_impl(ctx);
  auto& arena = impl->arena;
  while (arena.watermark > watermark) {
    arena.watermark--;
    size_t block = arena.watermark / HandleArena::kBlockSize;
    size_t index = arena.watermark % HandleArena::kBlockSize;
    auto& slot = arena.blocks[block][index];
    unregister_handle_repr(ctx, reinterpret_cast<uintptr_t>(&slot.object));
    unregister_handle_repr(ctx, static_cast<uintptr_t>(slot.object.tagged_map.value));
    if (slot.owns_root && slot.object.contents.root_id != 0 && bridge) {
      bridge->root_drop(slot.object.contents.root_id);
    }
    slot = shim::HandleSlot{};
  }
}

uintptr_t local_from_root(RasterV8ContextState* ctx, uint64_t root_id, const shim::Map* map) {
  auto* slot = alloc_handle_slot(ctx);
  slot->object = shim::ObjectLayout(map, root_id);
  slot->object.tagged_map = shim::TaggedPointer(&slot->object);
  slot->tagged_value = shim::TaggedPointer(&slot->object);
  slot->owns_root = false;
  note_materialized_layout(&slot->object);
  register_handle_repr(ctx, reinterpret_cast<uintptr_t>(&slot->object), root_id);
  register_handle_repr(ctx, static_cast<uintptr_t>(slot->object.tagged_map.value), root_id);
  return reinterpret_cast<uintptr_t>(slot->tagged_value.slot());
}

uint64_t root_from_local(uintptr_t tagged) {
  shim::TaggedPointer ptr = shim::TaggedPointer::fromRaw(tagged);
  if (ptr.isSmi()) {
    return 0;
  }
  auto* layout = ptr.getPtr<shim::ObjectLayout>();
  if (!layout) {
    if (const RasterV8BridgeV1* bridge = raster_v8_bridge()) {
      bridge->fatal("root_from_local", "invalid local handle");
    }
    return 0;
  }
  if (layout->contents.root_id != 0) {
    return layout->contents.root_id;
  }
  if (auto* ctx = raster_v8_current_context()) {
    if (uint64_t root_id = resolve_root_from_repr(ctx, reinterpret_cast<uintptr_t>(layout))) {
      layout->contents.root_id = root_id;
      return root_id;
    }
    if (uint64_t root_id =
            resolve_root_from_repr(ctx, static_cast<uintptr_t>(layout->tagged_map.value))) {
      layout->contents.root_id = root_id;
      return root_id;
    }
  }
  return layout->contents.root_id;
}

void persistent_counts_for_context(IsolateImpl* isolate,
                                   uintptr_t context_key,
                                   size_t* strong_out,
                                   size_t* weak_out) {
  size_t strong_count = 0;
  size_t weak_count = 0;
  if (isolate) {
    for (const auto& [cell, slot] : isolate->persistents) {
      (void)cell;
      if (slot.context_key != context_key) {
        continue;
      }
      if (slot.is_weak || slot.root_id == 0) {
        weak_count++;
      } else {
        strong_count++;
      }
    }
  }
  if (strong_out) {
    *strong_out = strong_count;
  }
  if (weak_out) {
    *weak_out = weak_count;
  }
}

size_t dispose_strong_context_persistents(IsolateImpl* isolate, uintptr_t context_key) {
  if (!isolate) {
    return 0;
  }
  const RasterV8BridgeV1* b = raster_v8_bridge();
  size_t weak_remaining = 0;
  for (auto it = isolate->persistents.begin(); it != isolate->persistents.end();) {
    if (it->second.context_key != context_key) {
      ++it;
      continue;
    }
    if (it->second.is_weak || it->second.root_id == 0) {
      weak_remaining++;
      ++it;
      continue;
    }
    if (b && b->root_drop) {
      b->root_drop(it->second.root_id);
    }
    auto* layout = reinterpret_cast<shim::ObjectLayout*>(*it->first);
    const auto layout_addr = reinterpret_cast<uintptr_t>(layout);
    isolate->layout_to_root.erase(layout_addr);
    isolate->layout_to_function_id.erase(layout_addr);
    delete layout;
    delete it->first;
    it = isolate->persistents.erase(it);
  }
  return weak_remaining;
}

size_t dispose_weak_context_persistents(IsolateImpl* isolate, uintptr_t context_key) {
  if (!isolate) {
    return 0;
  }
  size_t disposed = 0;
  for (auto it = isolate->persistents.begin(); it != isolate->persistents.end();) {
    if (it->second.context_key != context_key || !it->second.is_weak) {
      ++it;
      continue;
    }
    auto* layout = reinterpret_cast<shim::ObjectLayout*>(*it->first);
    const auto layout_addr = reinterpret_cast<uintptr_t>(layout);
    isolate->layout_to_root.erase(layout_addr);
    isolate->layout_to_function_id.erase(layout_addr);
    delete layout;
    delete it->first;
    it = isolate->persistents.erase(it);
    disposed++;
  }
  return disposed;
}

void dispose_context_persistents(IsolateImpl* isolate, uintptr_t context_key) {
  size_t weak_remaining = dispose_strong_context_persistents(isolate, context_key);
  if (weak_remaining != 0) {
    fprintf(stderr,
            "raster_v8: %zu weak persistent(s) remain for context %#lx\n",
            weak_remaining,
            static_cast<unsigned long>(context_key));
    abort();
  }
}

void dispose_isolate_persistents(IsolateImpl* isolate) {
  if (!isolate) {
    return;
  }
  if (!isolate->persistents.empty()) {
    fprintf(stderr,
            "raster_v8: %zu persistent(s) remain at isolate destroy\n",
            isolate->persistents.size());
    abort();
  }
  isolate->layout_to_root.clear();
  isolate->layout_to_function_id.clear();
}

}  // namespace raster_v8
