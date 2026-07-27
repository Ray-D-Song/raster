#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define RASTER_V8_BRIDGE_VERSION 1

typedef enum RasterV8Status {
  RASTER_V8_OK = 0,
  RASTER_V8_EXCEPTION = 1,
  RASTER_V8_UNSUPPORTED = 2,
  RASTER_V8_WRONG_THREAD = 3,
  RASTER_V8_ERROR = 4,
} RasterV8Status;

typedef struct RasterV8ContextState RasterV8ContextState;
typedef struct RasterV8IsolateState RasterV8IsolateState;

typedef RasterV8Status (*RasterV8RootDupFn)(uint64_t root_id, uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8RootDropFn)(uint64_t root_id);
typedef RasterV8Status (*RasterV8RootFromJsFn)(
    RasterV8ContextState* ctx,
    uint64_t js_value_tag,
    uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8RootToJsFn)(
    RasterV8ContextState* ctx,
    uint64_t root_id,
    uint64_t* out_js_value_tag);
typedef RasterV8Status (*RasterV8ThrowTypeErrorFn)(RasterV8ContextState* ctx, const char* message);
typedef void (*RasterV8FatalFn)(const char* api_name, const char* message);
typedef RasterV8Status (*RasterV8OddballRootFn)(
    RasterV8ContextState* ctx,
    int root_index,
    uint64_t* out_root_id);

typedef RasterV8Status (*RasterV8StringNewUtf8Fn)(
    RasterV8ContextState* ctx,
    const char* data,
    int length,
    uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8ObjectNewFn)(RasterV8ContextState* ctx, uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8ObjectSetFn)(
    RasterV8ContextState* ctx,
    uint64_t object_root_id,
    uint64_t key_root_id,
    uint64_t value_root_id);
typedef RasterV8Status (*RasterV8FunctionTemplateNewFn)(
    RasterV8ContextState* ctx,
    uint32_t template_id,
    void* callback,
    uint64_t data_root_id,
    uint64_t* out_template_id);
typedef RasterV8Status (*RasterV8FunctionTemplateGetFunctionFn)(
    RasterV8ContextState* ctx,
    uint32_t template_id,
    uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8DispatchFunctionFn)(
    RasterV8ContextState* ctx,
    uint32_t function_id,
    uint64_t receiver_root_id,
    uint64_t new_target_root_id,
    const uint64_t* arg_root_ids,
    int argc,
    uint64_t* out_result_root_id);
typedef RasterV8Status (*RasterV8RunModuleInitFn)(
    RasterV8ContextState* ctx,
    void (*init_fn)(void*, void*, void*),
    uint64_t exports_root_id,
    uint64_t module_root_id,
    uint64_t* out_exports_root_id);

typedef RasterV8Status (*RasterV8ObjectGetFn)(
    RasterV8ContextState* ctx,
    uint64_t object_root_id,
    uint64_t key_root_id,
    uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8ObjectGetIndexFn)(
    RasterV8ContextState* ctx,
    uint64_t object_root_id,
    uint32_t index,
    uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8ObjectSetIndexFn)(
    RasterV8ContextState* ctx,
    uint64_t object_root_id,
    uint32_t index,
    uint64_t value_root_id);
typedef RasterV8Status (*RasterV8ObjectDefineOwnPropertyFn)(
    RasterV8ContextState* ctx,
    uint64_t object_root_id,
    uint64_t key_root_id,
    uint64_t value_root_id,
    int attr,
    bool* out_ok);
typedef RasterV8Status (*RasterV8ObjectHasOwnPropertyFn)(
    RasterV8ContextState* ctx,
    uint64_t object_root_id,
    uint64_t key_root_id,
    bool* out_ok);
typedef RasterV8Status (*RasterV8ObjectGetPrototypeFn)(
    RasterV8ContextState* ctx,
    uint64_t object_root_id,
    uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8ArrayNewFn)(RasterV8ContextState* ctx, int length, uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8NumberNewFn)(RasterV8ContextState* ctx, double value, uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8BigIntNewFn)(RasterV8ContextState* ctx, int64_t value, uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8IntegerNewFn)(RasterV8ContextState* ctx, int value, uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8StringNewLatin1Fn)(
    RasterV8ContextState* ctx,
    const uint8_t* data,
    int length,
    uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8StringToUtf8Fn)(
    RasterV8ContextState* ctx,
    uint64_t value_root_id,
    char** out_ptr,
    size_t* out_len);
typedef RasterV8Status (*RasterV8StringFreeUtf8Fn)(RasterV8ContextState* ctx, char* ptr);
typedef RasterV8Status (*RasterV8FunctionCallFn)(
    RasterV8ContextState* ctx,
    uint64_t func_root_id,
    uint64_t recv_root_id,
    int argc,
    const uint64_t* arg_root_ids,
    uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8ThrowValueFn)(RasterV8ContextState* ctx, uint64_t value_root_id);
typedef RasterV8Status (*RasterV8NewExceptionFn)(
    RasterV8ContextState* ctx,
    uint64_t msg_root_id,
    int kind,
    uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8ExternalNewFn)(RasterV8ContextState* ctx, void* ptr, uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8InternalFieldSetFn)(
    RasterV8ContextState* ctx,
    uint64_t object_root_id,
    int index,
    void* ptr);
typedef RasterV8Status (*RasterV8InternalFieldGetFn)(
    RasterV8ContextState* ctx,
    uint64_t object_root_id,
    int index,
    void** out_ptr);
typedef RasterV8Status (*RasterV8SymbolIteratorFn)(RasterV8ContextState* ctx, uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8GetCreationContextFn)(
    RasterV8ContextState* ctx,
    uint64_t object_root_id,
    uint64_t* out_root_id);
typedef RasterV8Status (*RasterV8RegisterWeakCallbackFn)(
    RasterV8ContextState* ctx,
    uint64_t object_root_id,
    void* parameter,
    void (*callback)(const void* data, int parameter));
typedef RasterV8Status (*RasterV8GetContextRootFn)(RasterV8ContextState* ctx, uint64_t* out_root_id);

typedef struct RasterV8BridgeV1 {
  uint32_t version;
  uint32_t node_module_version;
  RasterV8RootDupFn root_dup;
  RasterV8RootDropFn root_drop;
  RasterV8RootFromJsFn root_from_js;
  RasterV8RootToJsFn root_to_js;
  RasterV8ThrowTypeErrorFn throw_type_error;
  RasterV8FatalFn fatal;
  RasterV8StringNewUtf8Fn string_new_utf8;
  RasterV8ObjectNewFn object_new;
  RasterV8ObjectSetFn object_set;
  RasterV8FunctionTemplateNewFn function_template_new;
  RasterV8FunctionTemplateGetFunctionFn function_template_get_function;
    RasterV8DispatchFunctionFn dispatch_function;
    RasterV8RunModuleInitFn run_module_init;
    RasterV8ObjectGetFn object_get;
    RasterV8ObjectGetIndexFn object_get_index;
    RasterV8ObjectSetIndexFn object_set_index;
    RasterV8ObjectDefineOwnPropertyFn object_define_own_property;
    RasterV8ObjectHasOwnPropertyFn object_has_own_property;
    RasterV8ObjectGetPrototypeFn object_get_prototype;
    RasterV8ArrayNewFn array_new;
    RasterV8NumberNewFn number_new;
    RasterV8BigIntNewFn bigint_new;
    RasterV8IntegerNewFn integer_new;
    RasterV8StringNewLatin1Fn string_new_latin1;
    RasterV8StringToUtf8Fn string_to_utf8;
    RasterV8StringFreeUtf8Fn string_free_utf8;
    RasterV8FunctionCallFn function_call;
    RasterV8ThrowValueFn throw_value;
    RasterV8NewExceptionFn new_exception;
    RasterV8ExternalNewFn external_new;
    RasterV8InternalFieldSetFn internal_field_set;
    RasterV8InternalFieldGetFn internal_field_get;
    RasterV8SymbolIteratorFn symbol_iterator;
    RasterV8GetCreationContextFn get_creation_context;
    RasterV8RegisterWeakCallbackFn register_weak_callback;
    RasterV8GetContextRootFn get_context_root;
} RasterV8BridgeV1;

RasterV8Status raster_v8_run_module_init(
    RasterV8ContextState* ctx,
    void* module,
    uint64_t exports_root_id,
    uint64_t module_root_id,
    uint64_t* out_exports_root_id);

void raster_v8_bind_bridge(const RasterV8BridgeV1* bridge);
void raster_v8_set_oddball_root_fn(RasterV8OddballRootFn fn);
const RasterV8BridgeV1* raster_v8_bridge(void);

RasterV8ContextState* raster_v8_current_context(void);
RasterV8IsolateState* raster_v8_current_isolate(void);
void raster_v8_set_current_context(RasterV8ContextState* ctx);
void raster_v8_set_current_isolate(RasterV8IsolateState* isolate);

RasterV8IsolateState* raster_v8_create_isolate(void);
void raster_v8_destroy_isolate(RasterV8IsolateState* isolate);
RasterV8ContextState* raster_v8_create_context(void);
void raster_v8_destroy_context(RasterV8ContextState* ctx);
void raster_v8_open_handle_scope(RasterV8ContextState* ctx);
void raster_v8_close_handle_scope(RasterV8ContextState* ctx);
void raster_v8_set_context_root_id(RasterV8ContextState* ctx, uint64_t root_id);
uint64_t raster_v8_context_root_id(RasterV8ContextState* ctx);
RasterV8Status raster_v8_oddball_root(RasterV8ContextState* ctx,
                                      int root_index,
                                      uint64_t* out_root_id);
RasterV8Status raster_v8_dispatch_callback(uint32_t function_id,
                                           uint64_t receiver_root,
                                           uint64_t new_target_root,
                                           const uint64_t* arg_roots,
                                           int argc,
                                           uint64_t* out_result_root);
uint32_t raster_v8_function_template_id(uint32_t function_id);
int32_t raster_v8_instance_internal_field_count(uint32_t template_id);
uint32_t raster_v8_function_prototype_template_id(uint32_t template_id);
void raster_v8_set_function_template_prototype_root(uint32_t template_id, uint64_t root_id);
uint64_t raster_v8_function_template_prototype_root(uint32_t template_id);
size_t raster_v8_object_template_property_count(uint32_t object_template_id);
int raster_v8_object_template_property_at(uint32_t object_template_id,
                                          size_t index,
                                          uint64_t* key_root,
                                          uint32_t* value_template_id);
uint32_t raster_v8_register_function_for_template(uint32_t template_id);
size_t raster_v8_object_template_native_property_count(uint32_t object_template_id);
int raster_v8_object_template_native_property_at(uint32_t object_template_id,
                                               size_t index,
                                               uint64_t* name_root,
                                               uint32_t* accessor_id);
RasterV8Status raster_v8_dispatch_accessor(uint32_t accessor_id,
                                           uint64_t receiver_root,
                                           void* embedder_override,
                                           uint64_t* out_result_root);
RasterV8Status raster_v8_object_internal_field_count(RasterV8ContextState* ctx,
                                                     uint64_t object_root,
                                                     int* out);
RasterV8Status raster_v8_object_reserve_internal_fields(RasterV8ContextState* ctx,
                                                      uint64_t object_root,
                                                      int count);
size_t raster_v8_pending_modules_count(void);
void* raster_v8_take_pending_module(size_t index);

void raster_v8_force_link(void);

enum RasterV8ValueLayoutKind : uint8_t {
  RASTER_V8_LAYOUT_OBJECT = 0,
  RASTER_V8_LAYOUT_STRING = 1,
  RASTER_V8_LAYOUT_ODDBALL = 2,
  RASTER_V8_LAYOUT_HEAP_NUMBER = 3,
  RASTER_V8_LAYOUT_INT32_SMI = 4,
};

RasterV8Status raster_v8_value_layout_kind(RasterV8ContextState* ctx, uint64_t root,
                                           uint8_t* out_kind, int32_t* out_smi);
RasterV8Status raster_v8_value_is_object(RasterV8ContextState* ctx, uint64_t root, bool* out);
RasterV8Status raster_v8_value_is_array(RasterV8ContextState* ctx, uint64_t root, bool* out);
RasterV8Status raster_v8_value_is_function(RasterV8ContextState* ctx, uint64_t root, bool* out);
RasterV8Status raster_v8_value_is_number(RasterV8ContextState* ctx, uint64_t root, bool* out);
RasterV8Status raster_v8_value_is_int32(RasterV8ContextState* ctx, uint64_t root, bool* out);
RasterV8Status raster_v8_value_is_bigint(RasterV8ContextState* ctx, uint64_t root, bool* out);
RasterV8Status raster_v8_value_is_boolean(RasterV8ContextState* ctx, uint64_t root, bool* out);
RasterV8Status raster_v8_value_to_boolean(RasterV8ContextState* ctx, uint64_t root, bool* out);
RasterV8Status raster_v8_value_strict_equals(RasterV8ContextState* ctx, uint64_t a, uint64_t b, bool* out);
RasterV8Status raster_v8_value_to_float64(RasterV8ContextState* ctx, uint64_t root, double* out);
RasterV8Status raster_v8_value_to_int32(RasterV8ContextState* ctx, uint64_t root, int32_t* out);
RasterV8Status raster_v8_value_to_int64(RasterV8ContextState* ctx, uint64_t root, int64_t* out, bool* lossless);
RasterV8Status raster_v8_array_length(RasterV8ContextState* ctx, uint64_t root, uint32_t* out);
RasterV8Status raster_v8_function_new_instance(RasterV8ContextState* ctx, uint64_t func_root, int argc,
                                               const uint64_t* args, uint64_t* out);
RasterV8Status raster_v8_buffer_new_copy(RasterV8ContextState* ctx, const uint8_t* data, size_t len,
                                        uint64_t* out);
RasterV8Status raster_v8_buffer_data(RasterV8ContextState* ctx, uint64_t root, uint8_t** out_ptr,
                                     size_t* out_len);
RasterV8Status raster_v8_internal_field_get(RasterV8ContextState* ctx, uint64_t object_root, int index,
                                            void** out_ptr);
RasterV8Status raster_v8_internal_field_set(RasterV8ContextState* ctx, uint64_t object_root, int index,
                                            void* ptr);
RasterV8Status raster_v8_root_id_for_js_object(RasterV8ContextState* ctx, void* object_ptr,
                                               uint64_t* out_root_id);
RasterV8Status raster_v8_root_id_for_persistent_layout(void* layout, uint64_t* out_root_id);
void raster_v8_register_layout_root(void* layout, uint64_t root_id);
void raster_v8_register_layout_function_id(void* layout, uint32_t function_id);
RasterV8Status raster_v8_function_id_for_layout(void* layout, uint32_t* out_function_id);
RasterV8Status raster_v8_function_id_for_root(RasterV8ContextState* ctx, uint64_t root_id,
                                              uint32_t* out_function_id);
RasterV8Status raster_v8_function_root_for_id(RasterV8ContextState* ctx, uint32_t function_id,
                                             uint64_t* out_root_id);
RasterV8Status raster_v8_buffer_has_instance(RasterV8ContextState* ctx, uint64_t root, bool* out);
void raster_v8_add_env_cleanup_hook(RasterV8IsolateState* isolate, void (*cb)(void*), void* arg);

#ifdef __cplusplus
}
#endif
