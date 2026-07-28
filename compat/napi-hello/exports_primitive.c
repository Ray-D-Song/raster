#include <node_api.h>

static napi_value Init(napi_env env, napi_value exports) {
  napi_value result;
  if (napi_create_string_utf8(env, "primitive-export", NAPI_AUTO_LENGTH, &result) != napi_ok) {
    return NULL;
  }
  (void)exports;
  return result;
}

NAPI_MODULE(NODE_GYP_MODULE_NAME, Init)
