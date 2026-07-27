#include "internal.h"
#include "v8_local_helpers.h"

#include <node.h>
#include <v8-context.h>
#include <v8-local-handle.h>
#include <v8-object.h>

extern "C" RasterV8Status raster_v8_run_module_init(
    RasterV8ContextState* ctx,
    void* module_opaque,
    uint64_t exports_root_id,
    uint64_t module_root_id,
    uint64_t* out_exports_root_id) {
  const RasterV8BridgeV1* bridge = raster_v8_bridge();
  if (!bridge || !ctx || !module_opaque || !out_exports_root_id) {
    return RASTER_V8_ERROR;
  }

  auto* node_mod = reinterpret_cast<node::node_module*>(module_opaque);
  raster_v8_set_current_context(ctx);
  raster_v8_open_handle_scope(ctx);

  v8::Isolate* isolate = reinterpret_cast<v8::Isolate*>(raster_v8_current_isolate());
  v8::HandleScope scope(isolate);

  auto* exports_slot = raster_v8::alloc_handle_slot(ctx);
  exports_slot->object = raster_v8::shim::ObjectLayout(
      const_cast<raster_v8::shim::Map*>(&raster_v8::shim::Map::object_map()), exports_root_id);
  auto* module_slot = raster_v8::alloc_handle_slot(ctx);
  module_slot->object = raster_v8::shim::ObjectLayout(
      const_cast<raster_v8::shim::Map*>(&raster_v8::shim::Map::object_map()), module_root_id);
  auto* context_slot = raster_v8::alloc_handle_slot(ctx);
  context_slot->object = raster_v8::shim::ObjectLayout(
      const_cast<raster_v8::shim::Map*>(&raster_v8::shim::Map::object_map()),
      raster_v8::ctx_impl(ctx)->context_root_id);

  v8::Local<v8::Object> exports =
      v8::MakeLocalFromObject<v8::Object>(isolate, &exports_slot->object);
  v8::Local<v8::Value> module_obj =
      v8::MakeLocalFromObject<v8::Value>(isolate, &module_slot->object);
  v8::Local<v8::Context> context =
      v8::MakeLocalFromObject<v8::Context>(isolate, &context_slot->object);

  if (node_mod->nm_context_register_func) {
    node_mod->nm_context_register_func(exports, module_obj, context, node_mod->nm_priv);
  } else if (node_mod->nm_register_func) {
    node_mod->nm_register_func(exports, module_obj, nullptr);
  } else {
    raster_v8_close_handle_scope(ctx);
    return RASTER_V8_ERROR;
  }

  *out_exports_root_id = exports_slot->object.contents.root_id;
  raster_v8_close_handle_scope(ctx);
  return RASTER_V8_OK;
}
