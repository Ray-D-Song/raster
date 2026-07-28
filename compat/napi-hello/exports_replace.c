#include <node_api.h>

static napi_value ReplFn(napi_env env, napi_callback_info info) {
  (void)info;
  napi_value result;
  napi_create_string_utf8(env, "fn-ok", NAPI_AUTO_LENGTH, &result);
  return result;
}

static napi_value Getter(napi_env env, napi_callback_info info) {
  (void)info;
  napi_value result;
  napi_create_string_utf8(env, "getter-ok", NAPI_AUTO_LENGTH, &result);
  return result;
}

static napi_value Init(napi_env env, napi_value exports) {
  napi_status status;
  napi_value replacement;
  status = napi_create_object(env, &replacement);
  if (status != napi_ok) {
    return NULL;
  }

  napi_value fn;
  status = napi_create_function(env, "replFn", NAPI_AUTO_LENGTH, ReplFn, NULL, &fn);
  if (status != napi_ok) {
    return NULL;
  }
  status = napi_set_named_property(env, replacement, "replFn", fn);
  if (status != napi_ok) {
    return NULL;
  }

  napi_value num;
  status = napi_create_int32(env, 42, &num);
  if (status != napi_ok) {
    return NULL;
  }
  status = napi_set_named_property(env, replacement, "value", num);
  if (status != napi_ok) {
    return NULL;
  }

  napi_value instance;
  status = napi_create_object(env, &instance);
  if (status != napi_ok) {
    return NULL;
  }
  napi_value marker;
  status = napi_create_string_utf8(env, "instance", NAPI_AUTO_LENGTH, &marker);
  if (status != napi_ok) {
    return NULL;
  }
  status = napi_set_named_property(env, instance, "kind", marker);
  if (status != napi_ok) {
    return NULL;
  }
  status = napi_set_named_property(env, replacement, "instance", instance);
  if (status != napi_ok) {
    return NULL;
  }

  napi_value sym_desc;
  status = napi_create_string_utf8(env, "tag", NAPI_AUTO_LENGTH, &sym_desc);
  if (status != napi_ok) {
    return NULL;
  }
  napi_value sym;
  status = napi_create_symbol(env, sym_desc, &sym);
  if (status != napi_ok) {
    return NULL;
  }
  napi_value sym_val;
  status = napi_create_string_utf8(env, "symval", NAPI_AUTO_LENGTH, &sym_val);
  if (status != napi_ok) {
    return NULL;
  }
  status = napi_set_property(env, replacement, sym, sym_val);
  if (status != napi_ok) {
    return NULL;
  }

  napi_property_descriptor getter_desc = {
      "fromGetter",
      NULL,
      NULL,
      Getter,
      NULL,
      NULL,
      napi_default,
      NULL,
  };
  status = napi_define_properties(env, replacement, 1, &getter_desc);
  if (status != napi_ok) {
    return NULL;
  }

  napi_value identity;
  status = napi_create_string_utf8(env, "replacement-exports-v1", NAPI_AUTO_LENGTH, &identity);
  if (status != napi_ok) {
    return NULL;
  }
  status = napi_set_named_property(env, replacement, "__identity", identity);
  if (status != napi_ok) {
    return NULL;
  }

  (void)exports;
  return replacement;
}

NAPI_MODULE(NODE_GYP_MODULE_NAME, Init)
