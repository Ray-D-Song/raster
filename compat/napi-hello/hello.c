#include <node_api.h>

static napi_value Hello(napi_env env, napi_callback_info info) {
  napi_status status;
  napi_value world;
  status = napi_create_string_utf8(env, "world", NAPI_AUTO_LENGTH, &world);
  if (status != napi_ok) {
    return NULL;
  }
  return world;
}

static napi_value Init(napi_env env, napi_value exports) {
  napi_status status;
  napi_property_descriptor desc = {
    "hello",
    NULL,
    Hello,
    NULL,
    NULL,
    NULL,
    napi_default,
    NULL,
  };
  status = napi_define_properties(env, exports, 1, &desc);
  if (status != napi_ok) {
    return NULL;
  }
  return exports;
}

NAPI_MODULE(NODE_GYP_MODULE_NAME, Init)
