#include "template_registry.h"
#include "v8_bridge_helpers.h"

#include <v8.h>

namespace v8 {

Local<Object> Object::New(Isolate* isolate) {
  const RasterV8BridgeV1* b = raster_v8::bridge();
  auto* ctx = raster_v8::bridge_ctx();
  if (!b || !ctx || !b->object_new) {
    return Local<Object>();
  }
  uint64_t root_id = 0;
  if (b->object_new(ctx, &root_id) != RASTER_V8_OK) {
    return Local<Object>();
  }
  return raster_v8::local_from_root<Object>(isolate, root_id, &raster_v8::shim::Map::object_map());
}

MaybeLocal<Value> Object::Get(Local<Context> context, Local<Value> key) {
  (void)context;
  const RasterV8BridgeV1* b = raster_v8::bridge();
  auto* ctx = raster_v8::bridge_ctx();
  if (!b || !ctx || !b->object_get) {
    return MaybeLocal<Value>();
  }
  uint64_t out = 0;
  if (b->object_get(ctx, raster_v8::root_from_object(this), raster_v8::root_from_local(key),
                    &out) != RASTER_V8_OK) {
    return MaybeLocal<Value>();
  }
  return raster_v8::local_value_from_root(Isolate::GetCurrent(), out);
}

MaybeLocal<Value> Object::Get(Local<Context> context, uint32_t index) {
  (void)context;
  const RasterV8BridgeV1* b = raster_v8::bridge();
  auto* ctx = raster_v8::bridge_ctx();
  if (!b || !ctx || !b->object_get_index) {
    return MaybeLocal<Value>();
  }
  uint64_t out = 0;
  if (b->object_get_index(ctx, raster_v8::root_from_object(this), index, &out) !=
      RASTER_V8_OK) {
    return MaybeLocal<Value>();
  }
  return raster_v8::local_value_from_root(Isolate::GetCurrent(), out);
}

Maybe<bool> Object::Set(Local<Context> context, uint32_t index, Local<Value> value) {
  (void)context;
  const RasterV8BridgeV1* b = raster_v8::bridge();
  auto* ctx = raster_v8::bridge_ctx();
  if (!b || !ctx || !b->object_set_index) {
    return Nothing<bool>();
  }
  if (b->object_set_index(ctx, raster_v8::root_from_object(this), index,
                          raster_v8::root_from_local(value)) != RASTER_V8_OK) {
    return Nothing<bool>();
  }
  return Just(true);
}

Maybe<bool> Object::DefineOwnProperty(Local<Context> context,
                                      Local<Name> key,
                                      Local<Value> value,
                                      PropertyAttribute attributes) {
  (void)context;
  const RasterV8BridgeV1* b = raster_v8::bridge();
  auto* ctx = raster_v8::bridge_ctx();
  if (!b || !ctx || !b->object_define_own_property) {
    return Nothing<bool>();
  }
  bool ok = false;
  if (b->object_define_own_property(ctx, raster_v8::root_from_object(this),
                                    raster_v8::root_from_local(*reinterpret_cast<Local<Value>*>(&key)),
                                    raster_v8::root_from_local(value),
                                    static_cast<int>(attributes), &ok) != RASTER_V8_OK) {
    return Nothing<bool>();
  }
  return Just(ok);
}

Maybe<bool> Object::HasOwnProperty(Local<Context> context, Local<Name> key) {
  (void)context;
  const RasterV8BridgeV1* b = raster_v8::bridge();
  auto* ctx = raster_v8::bridge_ctx();
  if (!b || !ctx || !b->object_has_own_property) {
    return Nothing<bool>();
  }
  bool ok = false;
  if (b->object_has_own_property(ctx, raster_v8::root_from_object(this),
                                 raster_v8::root_from_local(key), &ok) != RASTER_V8_OK) {
    return Nothing<bool>();
  }
  return Just(ok);
}

Local<Value> Object::GetPrototype() {
  const RasterV8BridgeV1* b = raster_v8::bridge();
  auto* ctx = raster_v8::bridge_ctx();
  if (!b || !ctx || !b->object_get_prototype) {
    return Local<Value>();
  }
  uint64_t out = 0;
  if (b->object_get_prototype(ctx, raster_v8::root_from_object(this), &out) != RASTER_V8_OK) {
    return Local<Value>();
  }
  return raster_v8::local_value_from_root(Isolate::GetCurrent(), out);
}

MaybeLocal<Context> Object::GetCreationContext() {
  const RasterV8BridgeV1* b = raster_v8::bridge();
  auto* ctx = raster_v8::bridge_ctx();
  if (!b || !ctx || !b->get_context_root) {
    return MaybeLocal<Context>();
  }
  uint64_t out = 0;
  if (b->get_context_root(ctx, &out) != RASTER_V8_OK) {
    return MaybeLocal<Context>();
  }
  return raster_v8::local_from_root<Context>(Isolate::GetCurrent(), out,
                                             &raster_v8::shim::Map::object_map());
}

void Object::SetAlignedPointerInInternalField(int index, void* value) {
  auto* ctx = raster_v8::bridge_ctx();
  if (!ctx) {
    return;
  }
  auto* layout = raster_v8::layout_from_slot(
      reinterpret_cast<v8::internal::Address*>(this));
  if (layout && index == 0) {
    layout->embedder_slot_0 = value;
  }
  uint64_t root_id = raster_v8::root_from_object(this);
  if (root_id == 0) {
    raster_v8_root_id_for_js_object(ctx, const_cast<void*>(static_cast<const void*>(this)), &root_id);
  }
  raster_v8_internal_field_set(ctx, root_id, index, value);
}

void* Object::SlowGetAlignedPointerFromInternalField(int index) {
  auto* ctx = raster_v8::bridge_ctx();
  if (!ctx) {
    return nullptr;
  }
  auto* layout = raster_v8::layout_from_slot(
      reinterpret_cast<v8::internal::Address*>(this));
  if (layout && index == 0 && layout->embedder_slot_0 != nullptr) {
    return layout->embedder_slot_0;
  }
  void* out = nullptr;
  uint64_t root_id = raster_v8::root_from_object(this);
  if (root_id == 0) {
    raster_v8_root_id_for_js_object(ctx, const_cast<void*>(static_cast<const void*>(this)), &root_id);
  }
  if (raster_v8_internal_field_get(ctx, root_id, index, &out) != RASTER_V8_OK) {
    return nullptr;
  }
  return out;
}

int Object::InternalFieldCount() const {
  auto* ctx = raster_v8::bridge_ctx();
  if (!ctx) {
    return 0;
  }
  int out = 0;
  if (raster_v8_object_internal_field_count(ctx, raster_v8::root_from_object(this), &out) !=
      RASTER_V8_OK) {
    return 0;
  }
  return out;
}

void ObjectTemplate::SetInternalFieldCount(int value) {
  auto* rec = raster_v8::TemplateRegistry::instance().object_template_at(
      static_cast<uint32_t>(raster_v8::root_from_object(this)));
  if (rec) {
    rec->internal_field_count = value;
  }
}

}  // namespace v8
