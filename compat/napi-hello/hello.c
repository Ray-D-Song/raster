#include <node_api.h>
#include <pthread.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static napi_value Hello(napi_env env, napi_callback_info info) {
  napi_status status;
  napi_value world;
  status = napi_create_string_utf8(env, "world", NAPI_AUTO_LENGTH, &world);
  if (status != napi_ok) {
    return NULL;
  }
  return world;
}

static int g_finalize_count = 0;

static void wrap_finalize(napi_env env, void* data, void* hint) {
  (void)env;
  (void)data;
  (void)hint;
  g_finalize_count++;
}

static napi_value RemoveWrapTest(napi_env env, napi_callback_info info) {
  (void)info;
  g_finalize_count = 0;
  napi_status status;
  napi_value obj;
  status = napi_create_object(env, &obj);
  if (status != napi_ok) {
    return NULL;
  }
  status = napi_wrap(env, obj, NULL, wrap_finalize, NULL, NULL);
  if (status != napi_ok) {
    return NULL;
  }
  void* native = NULL;
  status = napi_remove_wrap(env, obj, &native);
  if (status != napi_ok) {
    return NULL;
  }
  napi_value count;
  status = napi_create_int32(env, g_finalize_count, &count);
  if (status != napi_ok) {
    return NULL;
  }
  return count;
}

typedef struct {
  napi_env env;
  napi_ref callback_ref;
  napi_async_work async_work;
  int result;
} AsyncWorkData;

static void async_execute(napi_env env, void* data) {
  (void)env;
  AsyncWorkData* work = (AsyncWorkData*)data;
  work->result = 42;
}

static void async_complete(napi_env env, napi_status status, void* data) {
  AsyncWorkData* work = (AsyncWorkData*)data;
  if (status != napi_ok) {
    free(work);
    return;
  }
  napi_value callback;
  napi_status s = napi_get_reference_value(env, work->callback_ref, &callback);
  if (s != napi_ok) {
    napi_delete_reference(env, work->callback_ref);
    free(work);
    return;
  }
  napi_value undefined;
  napi_get_undefined(env, &undefined);
  napi_value result;
  napi_create_int32(env, work->result, &result);
  napi_value argv[1] = {result};
  napi_call_function(env, undefined, callback, 1, argv, NULL);
  napi_delete_reference(env, work->callback_ref);
  napi_delete_async_work(env, work->async_work);
  free(work);
}

static napi_value QueueAsyncWork(napi_env env, napi_callback_info info) {
  size_t argc = 1;
  napi_value argv[1];
  napi_status status = napi_get_cb_info(env, info, &argc, argv, NULL, NULL);
  if (status != napi_ok || argc < 1) {
    return NULL;
  }
  AsyncWorkData* work = calloc(1, sizeof(AsyncWorkData));
  if (!work) {
    return NULL;
  }
  work->env = env;
  status = napi_create_reference(env, argv[0], 1, &work->callback_ref);
  if (status != napi_ok) {
    free(work);
    return NULL;
  }
  napi_value resource_name;
  napi_create_string_utf8(env, "compat-async", NAPI_AUTO_LENGTH, &resource_name);
  napi_async_work async_work;
  status = napi_create_async_work(
      env, NULL, resource_name, async_execute, async_complete, work, &async_work);
  if (status != napi_ok) {
    napi_delete_reference(env, work->callback_ref);
    free(work);
    return NULL;
  }
  work->async_work = async_work;
  status = napi_queue_async_work(env, async_work);
  if (status != napi_ok) {
    napi_delete_async_work(env, async_work);
    napi_delete_reference(env, work->callback_ref);
    free(work);
    return NULL;
  }
  napi_value undefined;
  napi_get_undefined(env, &undefined);
  return undefined;
}

typedef struct {
  napi_threadsafe_function tsfn;
  int value;
} TsfnThreadArg;

static void tsfn_call_js(napi_env env, napi_value js_callback, void* context, void* data) {
  (void)context;
  TsfnThreadArg* arg = (TsfnThreadArg*)data;
  napi_value undefined;
  napi_get_undefined(env, &undefined);
  napi_value result;
  napi_create_int32(env, arg->value, &result);
  napi_value argv[1] = {result};
  napi_call_function(env, undefined, js_callback, 1, argv, NULL);
  free(arg);
}

static void* tsfn_thread_main(void* arg) {
  TsfnThreadArg* payload = (TsfnThreadArg*)arg;
  napi_call_threadsafe_function(
      payload->tsfn, payload, napi_tsfn_blocking);
  return NULL;
}

static napi_value CallTsfnFromThread(napi_env env, napi_callback_info info) {
  size_t argc = 1;
  napi_value argv[1];
  napi_status status = napi_get_cb_info(env, info, &argc, argv, NULL, NULL);
  if (status != napi_ok || argc < 1) {
    return NULL;
  }
  napi_value resource_name;
  napi_create_string_utf8(env, "compat-tsfn", NAPI_AUTO_LENGTH, &resource_name);
  napi_threadsafe_function tsfn;
  status = napi_create_threadsafe_function(
      env,
      argv[0],
      NULL,
      resource_name,
      0,
      1,
      NULL,
      NULL,
      NULL,
      tsfn_call_js,
      &tsfn);
  if (status != napi_ok) {
    return NULL;
  }
  TsfnThreadArg* payload = calloc(1, sizeof(TsfnThreadArg));
  if (!payload) {
    napi_release_threadsafe_function(tsfn, napi_tsfn_release);
    return NULL;
  }
  payload->tsfn = tsfn;
  payload->value = 99;
  pthread_t thread;
  if (pthread_create(&thread, NULL, tsfn_thread_main, payload) != 0) {
    free(payload);
    napi_release_threadsafe_function(tsfn, napi_tsfn_release);
    return NULL;
  }
  pthread_join(thread, NULL);
  napi_release_threadsafe_function(tsfn, napi_tsfn_release);
  napi_value undefined;
  napi_get_undefined(env, &undefined);
  return undefined;
}

static void timespec_sleep_ms(int ms) {
  struct timespec ts;
  ts.tv_sec = ms / 1000;
  ts.tv_nsec = (long)(ms % 1000) * 1000000L;
  nanosleep(&ts, NULL);
}

static napi_threadsafe_function g_stored_tsfn = NULL;

static void noop_tsfn_call_js(napi_env env, napi_value js_callback, void* context, void* data) {
  (void)env;
  (void)js_callback;
  (void)context;
  (void)data;
}

static napi_value CreateStoredTsfn(napi_env env, napi_callback_info info) {
  size_t argc = 1;
  napi_value argv[1];
  napi_status status = napi_get_cb_info(env, info, &argc, argv, NULL, NULL);
  if (status != napi_ok || argc < 1) {
    return NULL;
  }
  napi_value resource_name;
  napi_create_string_utf8(env, "stored-tsfn", NAPI_AUTO_LENGTH, &resource_name);
  status = napi_create_threadsafe_function(
      env,
      argv[0],
      NULL,
      resource_name,
      0,
      1,
      NULL,
      NULL,
      NULL,
      noop_tsfn_call_js,
      &g_stored_tsfn);
  if (status != napi_ok) {
    return NULL;
  }
  napi_value undefined;
  napi_get_undefined(env, &undefined);
  return undefined;
}

static napi_value UnrefStoredTsfn(napi_env env, napi_callback_info info) {
  (void)info;
  if (g_stored_tsfn == NULL) {
    return NULL;
  }
  napi_status status = napi_unref_threadsafe_function(env, g_stored_tsfn);
  if (status != napi_ok) {
    return NULL;
  }
  napi_value undefined;
  napi_get_undefined(env, &undefined);
  return undefined;
}

static napi_value ReleaseStoredTsfn(napi_env env, napi_callback_info info) {
  (void)info;
  if (g_stored_tsfn == NULL) {
    return NULL;
  }
  napi_status status = napi_release_threadsafe_function(g_stored_tsfn, napi_tsfn_release);
  g_stored_tsfn = NULL;
  if (status != napi_ok) {
    return NULL;
  }
  napi_value undefined;
  napi_get_undefined(env, &undefined);
  return undefined;
}

// One allocation: worker posts this payload; JS callback frees it on success.
// On napi_closing / other non-ok status, ownership stays with the worker.
typedef struct {
  napi_threadsafe_function tsfn;
  int value;
} DelayedThreadArg;

static void delayed_tsfn_call_js(napi_env env, napi_value js_callback, void* context, void* data) {
  (void)context;
  DelayedThreadArg* payload = (DelayedThreadArg*)data;
  napi_value undefined;
  napi_get_undefined(env, &undefined);
  napi_value result;
  napi_create_int32(env, payload->value, &result);
  napi_value argv[1] = {result};
  napi_call_function(env, undefined, js_callback, 1, argv, NULL);
  napi_threadsafe_function tsfn = payload->tsfn;
  free(payload);
  napi_release_threadsafe_function(tsfn, napi_tsfn_release);
}

static void* delayed_tsfn_thread_main(void* arg) {
  DelayedThreadArg* payload = (DelayedThreadArg*)arg;
  timespec_sleep_ms(30);
  napi_status status = napi_call_threadsafe_function(
      payload->tsfn, payload, napi_tsfn_blocking);

  if (status != napi_ok) {
    // Not enqueued: caller still owns the payload and must release the TSFN.
    napi_threadsafe_function tsfn = payload->tsfn;
    free(payload);
    napi_release_threadsafe_function(tsfn, napi_tsfn_release);
  }

  return NULL;
}

static napi_value DelayedTsfnUnrefExit(napi_env env, napi_callback_info info) {
  size_t argc = 1;
  napi_value argv[1];
  napi_status status = napi_get_cb_info(env, info, &argc, argv, NULL, NULL);
  if (status != napi_ok || argc < 1) {
    return NULL;
  }
  napi_value resource_name;
  napi_create_string_utf8(env, "delayed-tsfn", NAPI_AUTO_LENGTH, &resource_name);
  napi_threadsafe_function tsfn;
  status = napi_create_threadsafe_function(
      env,
      argv[0],
      NULL,
      resource_name,
      0,
      1,
      NULL,
      NULL,
      NULL,
      delayed_tsfn_call_js,
      &tsfn);
  if (status != napi_ok) {
    return NULL;
  }
  status = napi_unref_threadsafe_function(env, tsfn);
  if (status != napi_ok) {
    napi_release_threadsafe_function(tsfn, napi_tsfn_release);
    return NULL;
  }
  DelayedThreadArg* thread_arg = calloc(1, sizeof(DelayedThreadArg));
  if (!thread_arg) {
    napi_release_threadsafe_function(tsfn, napi_tsfn_release);
    return NULL;
  }
  thread_arg->tsfn = tsfn;
  thread_arg->value = 88;
  pthread_t thread;
  if (pthread_create(&thread, NULL, delayed_tsfn_thread_main, thread_arg) != 0) {
    free(thread_arg);
    napi_release_threadsafe_function(tsfn, napi_tsfn_release);
    return NULL;
  }
  pthread_detach(thread);
  napi_value undefined;
  napi_get_undefined(env, &undefined);
  return undefined;
}

typedef struct {
  char* module_path;
  napi_threadsafe_function tsfn;
} RequireAwaitPayload;

static void require_await_tsfn_call_js(
    napi_env env,
    napi_value js_callback,
    void* context,
    void* data) {
  (void)context;
  RequireAwaitPayload* payload = (RequireAwaitPayload*)data;
  napi_value global;
  napi_get_global(env, &global);
  napi_value require_fn;
  napi_get_named_property(env, global, "require", &require_fn);
  napi_value path_str;
  napi_create_string_utf8(env, payload->module_path, NAPI_AUTO_LENGTH, &path_str);
  napi_value undefined;
  napi_get_undefined(env, &undefined);
  napi_value module_exports;
  napi_call_function(env, undefined, require_fn, 1, &path_str, &module_exports);
  napi_value ok_val;
  napi_get_boolean(env, 1, &ok_val);
  napi_call_function(env, undefined, js_callback, 1, &ok_val, NULL);
  napi_threadsafe_function tsfn = payload->tsfn;
  free(payload->module_path);
  free(payload);
  napi_release_threadsafe_function(tsfn, napi_tsfn_release);
}

typedef struct {
  napi_threadsafe_function tsfn;
  char* module_path;
} RequireAwaitThreadArg;

static void* require_await_thread_main(void* arg) {
  RequireAwaitThreadArg* thread_arg = (RequireAwaitThreadArg*)arg;
  RequireAwaitPayload* payload = calloc(1, sizeof(RequireAwaitPayload));
  if (!payload) {
    free(thread_arg->module_path);
    free(thread_arg);
    return NULL;
  }
  payload->tsfn = thread_arg->tsfn;
  payload->module_path = thread_arg->module_path;
  napi_call_threadsafe_function(
      thread_arg->tsfn, payload, napi_tsfn_blocking);
  free(thread_arg);
  return NULL;
}

static napi_value TsfnRequireAwaitModule(napi_env env, napi_callback_info info) {
  size_t argc = 2;
  napi_value argv[2];
  napi_status status = napi_get_cb_info(env, info, &argc, argv, NULL, NULL);
  if (status != napi_ok || argc < 2) {
    return NULL;
  }
  size_t path_len = 0;
  status = napi_get_value_string_utf8(env, argv[1], NULL, 0, &path_len);
  if (status != napi_ok) {
    return NULL;
  }
  char* module_path = calloc(path_len + 1, 1);
  if (!module_path) {
    return NULL;
  }
  status = napi_get_value_string_utf8(env, argv[1], module_path, path_len + 1, &path_len);
  if (status != napi_ok) {
    free(module_path);
    return NULL;
  }
  napi_value resource_name;
  napi_create_string_utf8(env, "require-await-tsfn", NAPI_AUTO_LENGTH, &resource_name);
  napi_threadsafe_function tsfn;
  status = napi_create_threadsafe_function(
      env,
      argv[0],
      NULL,
      resource_name,
      0,
      1,
      NULL,
      NULL,
      NULL,
      require_await_tsfn_call_js,
      &tsfn);
  if (status != napi_ok) {
    free(module_path);
    return NULL;
  }
  RequireAwaitThreadArg* thread_arg = calloc(1, sizeof(RequireAwaitThreadArg));
  if (!thread_arg) {
    free(module_path);
    napi_release_threadsafe_function(tsfn, napi_tsfn_release);
    return NULL;
  }
  thread_arg->tsfn = tsfn;
  thread_arg->module_path = module_path;
  pthread_t thread;
  if (pthread_create(&thread, NULL, require_await_thread_main, thread_arg) != 0) {
    free(module_path);
    free(thread_arg);
    napi_release_threadsafe_function(tsfn, napi_tsfn_release);
    return NULL;
  }
  pthread_detach(thread);
  napi_value undefined;
  napi_get_undefined(env, &undefined);
  return undefined;
}

static napi_value Init(napi_env env, napi_value exports) {
  napi_status status;
  napi_property_descriptor desc[] = {
      {"hello", NULL, Hello, NULL, NULL, NULL, napi_default, NULL},
      {"removeWrapTest", NULL, RemoveWrapTest, NULL, NULL, NULL, napi_default, NULL},
      {"queueAsyncWork", NULL, QueueAsyncWork, NULL, NULL, NULL, napi_default, NULL},
      {"callTsfnFromThread", NULL, CallTsfnFromThread, NULL, NULL, NULL, napi_default, NULL},
      {"createStoredTsfn", NULL, CreateStoredTsfn, NULL, NULL, NULL, napi_default, NULL},
      {"unrefStoredTsfn", NULL, UnrefStoredTsfn, NULL, NULL, NULL, napi_default, NULL},
      {"releaseStoredTsfn", NULL, ReleaseStoredTsfn, NULL, NULL, NULL, napi_default, NULL},
      {"delayedTsfnUnrefExit", NULL, DelayedTsfnUnrefExit, NULL, NULL, NULL, napi_default, NULL},
      {"tsfnRequireAwaitModule", NULL, TsfnRequireAwaitModule, NULL, NULL, NULL, napi_default, NULL},
  };
  status = napi_define_properties(env, exports, 9, desc);
  if (status != napi_ok) {
    return NULL;
  }
  return exports;
}

NAPI_MODULE(NODE_GYP_MODULE_NAME, Init)
