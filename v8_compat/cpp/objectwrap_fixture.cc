#include "internal.h"
#include "raster_v8_bridge.h"
#include "v8_bridge_helpers.h"

#include <atomic>
#include <cstdint>

#include <v8-persistent-handle.h>
#include <v8-weak-callback-info.h>

namespace {

struct NativeWrap {
  std::atomic<int>* destructor_count;
  ~NativeWrap() {
    if (destructor_count) {
      destructor_count->fetch_add(1, std::memory_order_relaxed);
    }
  }
};

struct FixtureCounters;

struct WeakClosure {
  FixtureCounters* owner;
  std::atomic<int>* weak_callback_count;
  v8::Persistent<v8::Object>* persistent;
  NativeWrap native;
};

struct FixtureCounters {
  std::atomic<int> cleanup_count{0};
  std::atomic<int> weak_callback_count{0};
  std::atomic<int> destructor_count{0};
  v8::Persistent<v8::Object>* strong_persistent = nullptr;
  WeakClosure* weak_closure = nullptr;
  uint64_t strong_root_id = 0;
  uint64_t weak_root_id = 0;
};

static void fixture_weak_callback(const v8::WeakCallbackInfo<void>& data) {
  auto* closure = static_cast<WeakClosure*>(data.GetParameter());
  if (!closure) {
    return;
  }
  if (closure->weak_callback_count) {
    closure->weak_callback_count->fetch_add(1, std::memory_order_relaxed);
  }
  if (closure->persistent != nullptr) {
    closure->persistent->Reset();
    delete closure->persistent;
    closure->persistent = nullptr;
  }
  if (closure->owner != nullptr) {
    closure->owner->weak_closure = nullptr;
  }
  delete closure;
}

}  // namespace

extern "C" void raster_v8_test_objectwrap_fixture_cleanup_hook(void* arg) {
  auto* counters = static_cast<FixtureCounters*>(arg);
  counters->cleanup_count.fetch_add(1, std::memory_order_relaxed);
  if (counters->strong_persistent != nullptr && !counters->strong_persistent->IsEmpty()) {
    counters->strong_persistent->Reset();
    delete counters->strong_persistent;
    counters->strong_persistent = nullptr;
  }
}

extern "C" FixtureCounters* raster_v8_test_objectwrap_fixture_setup(
    RasterV8ContextState* ctx_state) {
  const RasterV8BridgeV1* bridge = raster_v8_bridge();
  auto* isolate = reinterpret_cast<v8::Isolate*>(raster_v8_current_isolate());
  if (!bridge || !ctx_state || !isolate) {
    return nullptr;
  }

  auto* counters = new FixtureCounters();
  auto* closure = new WeakClosure{
      counters,
      &counters->weak_callback_count,
      nullptr,
      NativeWrap{&counters->destructor_count},
  };
  counters->weak_closure = closure;

  if (bridge->object_new(ctx_state, &counters->strong_root_id) != RASTER_V8_OK ||
      counters->strong_root_id == 0) {
    delete closure;
    delete counters;
    return nullptr;
  }
  v8::Local<v8::Object> strong_object = raster_v8::local_from_root<v8::Object>(
      isolate, counters->strong_root_id, &raster_v8::shim::Map::object_map());
  counters->strong_persistent = new v8::Persistent<v8::Object>();
  counters->strong_persistent->Reset(isolate, strong_object);

  if (bridge->object_new(ctx_state, &counters->weak_root_id) != RASTER_V8_OK ||
      counters->weak_root_id == 0) {
    counters->strong_persistent->Reset();
    delete counters->strong_persistent;
    delete closure;
    delete counters;
    return nullptr;
  }
  v8::Local<v8::Object> weak_object = raster_v8::local_from_root<v8::Object>(
      isolate, counters->weak_root_id, &raster_v8::shim::Map::object_map());
  closure->persistent = new v8::Persistent<v8::Object>();
  closure->persistent->Reset(isolate, weak_object);
  closure->persistent->SetWeak(static_cast<void*>(closure),
                               fixture_weak_callback,
                               v8::WeakCallbackType::kParameter);

  return counters;
}

extern "C" void raster_v8_test_objectwrap_fixture_release_bridge_roots(
    RasterV8ContextState* ctx_state,
    FixtureCounters* counters) {
  const RasterV8BridgeV1* bridge = raster_v8_bridge();
  if (!bridge || !ctx_state || !counters) {
    return;
  }
  if (counters->strong_root_id != 0 && bridge->root_drop) {
    bridge->root_drop(counters->strong_root_id);
    counters->strong_root_id = 0;
  }
  if (counters->weak_root_id != 0 && bridge->root_drop) {
    bridge->root_drop(counters->weak_root_id);
    counters->weak_root_id = 0;
  }
}

extern "C" void raster_v8_test_objectwrap_fixture_read_counts(const FixtureCounters* counters,
                                                              int* cleanup_out,
                                                              int* weak_out,
                                                              int* destructor_out) {
  if (!counters) {
    return;
  }
  if (cleanup_out) {
    *cleanup_out = counters->cleanup_count.load(std::memory_order_relaxed);
  }
  if (weak_out) {
    *weak_out = counters->weak_callback_count.load(std::memory_order_relaxed);
  }
  if (destructor_out) {
    *destructor_out = counters->destructor_count.load(std::memory_order_relaxed);
  }
}

extern "C" void raster_v8_test_objectwrap_fixture_destroy(FixtureCounters* counters) {
  if (!counters) {
    return;
  }
  if (counters->strong_persistent != nullptr) {
    counters->strong_persistent->Reset();
    delete counters->strong_persistent;
  }
  if (counters->weak_closure != nullptr) {
    if (counters->weak_closure->persistent != nullptr) {
      counters->weak_closure->persistent->Reset();
      delete counters->weak_closure->persistent;
    }
    delete counters->weak_closure;
  }
  delete counters;
}

extern "C" int raster_v8_test_objectwrap_strong_reset_scrubs_layout_maps(
    RasterV8ContextState* ctx_state) {
  const RasterV8BridgeV1* bridge = raster_v8_bridge();
  auto* isolate = reinterpret_cast<v8::Isolate*>(raster_v8_current_isolate());
  auto* isolate_impl = raster_v8::iso_impl(
      reinterpret_cast<RasterV8IsolateState*>(raster_v8_current_isolate()));
  if (!bridge || !ctx_state || !isolate || !isolate_impl) {
    return 0;
  }

  uint64_t root_id = 0;
  if (bridge->object_new(ctx_state, &root_id) != RASTER_V8_OK || root_id == 0) {
    return 0;
  }
  v8::Local<v8::Object> object = raster_v8::local_from_root<v8::Object>(
      isolate, root_id, &raster_v8::shim::Map::object_map());

  v8::Persistent<v8::Object> persistent;
  persistent.Reset(isolate, object);

  uintptr_t layout_addr = 0;
  for (const auto& [cell, slot] : isolate_impl->persistents) {
    if (slot.root_id == root_id) {
      layout_addr = *cell;
      break;
    }
  }
  if (layout_addr == 0) {
    return 0;
  }

  raster_v8_register_layout_root(reinterpret_cast<void*>(layout_addr), root_id);
  raster_v8_register_layout_function_id(reinterpret_cast<void*>(layout_addr), 1);

  persistent.Reset();

  const bool maps_clean =
      isolate_impl->layout_to_root.find(layout_addr) == isolate_impl->layout_to_root.end() &&
      isolate_impl->layout_to_function_id.find(layout_addr) ==
          isolate_impl->layout_to_function_id.end();

  uint64_t second_root_id = 0;
  if (bridge->object_new(ctx_state, &second_root_id) == RASTER_V8_OK && second_root_id != 0) {
    v8::Local<v8::Object> second_object = raster_v8::local_from_root<v8::Object>(
        isolate, second_root_id, &raster_v8::shim::Map::object_map());
    v8::Persistent<v8::Object> second_persistent;
    second_persistent.Reset(isolate, second_object);
    second_persistent.Reset();
  }

  return maps_clean ? 1 : 0;
}
