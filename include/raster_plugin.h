#ifndef RASTER_PLUGIN_H
#define RASTER_PLUGIN_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct RasterPluginCall {
    uint64_t call_id;
    const char *plugin;
    const char *method;
    const char *args_json;
    void *context;
} RasterPluginCall;

typedef void (*RasterPluginHandler)(const RasterPluginCall *call);

bool raster_plugin_register_method(
    const char *plugin,
    const char *method,
    RasterPluginHandler handler,
    void *context
);

void raster_plugin_reply_ok(uint64_t call_id, const char *result_json);
void raster_plugin_reply_err(uint64_t call_id, const char *code, const char *message);
void raster_plugin_emit_event(const char *plugin, const char *event, const char *data_json);

#ifdef __cplusplus
}
#endif

#endif