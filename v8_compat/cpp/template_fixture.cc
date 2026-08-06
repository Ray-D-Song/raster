#include "function_registry.h"
#include "raster_v8_bridge.h"
#include "template_registry.h"
#include "v8_bridge_helpers.h"

#include <cstdint>

namespace {

static void noop_callback(const v8::FunctionCallbackInfo<v8::Value>& info) {
  (void)info;
}

}  // namespace

extern "C" uint32_t raster_v8_test_register_function_template() {
  uint32_t template_id =
      raster_v8::FunctionRegistry::instance().register_template(noop_callback, 0);
  return raster_v8::FunctionRegistry::instance().register_function(template_id);
}

extern "C" void raster_v8_test_register_template_property_with_missing_key(uint32_t function_id) {
  auto* fn = raster_v8::FunctionRegistry::instance().function_at(function_id);
  if (!fn) {
    return;
  }
  auto* fn_rec =
      raster_v8::TemplateRegistry::instance().function_template_at(fn->template_id);
  if (!fn_rec) {
    return;
  }
  auto* rec = raster_v8::TemplateRegistry::instance().object_template_at(
      fn_rec->prototype_template_id);
  if (!rec) {
    return;
  }
  uint32_t child_template_id =
      raster_v8::FunctionRegistry::instance().register_template(noop_callback, 0);
  rec->properties.emplace_back(
      UINT64_C(0xDEADBEEF),
      static_cast<uint64_t>(child_template_id),
      v8::None);
}

extern "C" RasterV8Status raster_v8_test_function_template_get_function(
    RasterV8ContextState* ctx_state,
    uint32_t function_id,
    uint64_t* out_root) {
  const RasterV8BridgeV1* bridge = raster_v8_bridge();
  if (!bridge || !ctx_state || !out_root || !bridge->function_template_get_function) {
    return RASTER_V8_ERROR;
  }
  return bridge->function_template_get_function(ctx_state, function_id, out_root);
}

extern "C" RasterV8Status raster_v8_test_function_root_for_id(
    RasterV8ContextState* ctx_state,
    uint32_t function_id,
    uint64_t* out_root) {
  return raster_v8_function_root_for_id(ctx_state, function_id, out_root);
}
