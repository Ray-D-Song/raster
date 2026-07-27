#include "internal.h"

#include "abi_137_generated.h"

#include <node.h>
#include <v8.h>

extern "C" void node_module_register(void* mod_opaque) {
  const RasterV8BridgeV1* bridge = raster_v8_bridge();
  if (!bridge) {
    return;
  }
  auto* mod = reinterpret_cast<node::node_module*>(mod_opaque);
  if (!mod) {
    bridge->fatal("node_module_register", "null module pointer");
    return;
  }
  if (!raster_v8_current_context()) {
    bridge->fatal("node_module_register", "no active native load context");
    return;
  }
  if (mod->nm_version != RASTER_V8_NODE_MODULE_VERSION) {
    char message[256];
    snprintf(message,
             sizeof(message),
             "module ABI %d does not match required %d",
             mod->nm_version,
             RASTER_V8_NODE_MODULE_VERSION);
    bridge->throw_type_error(raster_v8_current_context(), message);
    return;
  }
  raster_v8::push_pending_v8_module(mod);
}
