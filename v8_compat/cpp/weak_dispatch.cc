#include "internal.h"

#include <v8-weak-callback-info.h>

extern "C" int raster_v8_invoke_weak_callback_first_pass(void* callback_opaque,
                                                        void* parameter,
                                                        void** out_second_pass) {
  if (!callback_opaque) {
    return 0;
  }
  if (out_second_pass) {
    *out_second_pass = nullptr;
  }
  auto callback = reinterpret_cast<v8::WeakCallbackInfo<void>::Callback>(callback_opaque);
  auto* isolate = reinterpret_cast<v8::Isolate*>(raster_v8_current_isolate());
  if (!isolate) {
    return 0;
  }
  void* embedder_fields[v8::kEmbedderFieldsInWeakCallback] = {nullptr, nullptr};
  v8::WeakCallbackInfo<void>::Callback second_pass = nullptr;
  v8::WeakCallbackInfo<void> info(isolate, parameter, embedder_fields, &second_pass);
  callback(info);
  if (second_pass != nullptr && out_second_pass) {
    *out_second_pass = reinterpret_cast<void*>(second_pass);
    return 1;
  }
  return 0;
}

extern "C" void raster_v8_invoke_weak_callback_second_pass(void* callback_opaque, void* parameter) {
  if (!callback_opaque) {
    return;
  }
  auto callback = reinterpret_cast<v8::WeakCallbackInfo<void>::Callback>(callback_opaque);
  auto* isolate = reinterpret_cast<v8::Isolate*>(raster_v8_current_isolate());
  if (!isolate) {
    return;
  }
  void* embedder_fields[v8::kEmbedderFieldsInWeakCallback] = {nullptr, nullptr};
  v8::WeakCallbackInfo<void>::Callback unused = nullptr;
  v8::WeakCallbackInfo<void> info(isolate, parameter, embedder_fields, &unused);
  callback(info);
}

// Legacy single-shot dispatcher (first + inline second pass).
extern "C" void raster_v8_invoke_weak_callback(void* callback_opaque, void* parameter) {
  void* second_pass = nullptr;
  if (raster_v8_invoke_weak_callback_first_pass(callback_opaque, parameter, &second_pass) && second_pass) {
    raster_v8_invoke_weak_callback_second_pass(second_pass, parameter);
  }
}
