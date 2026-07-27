#include <node.h>
#include <v8.h>

#include <iostream>

using v8::Context;
using v8::Function;
using v8::FunctionCallbackInfo;
using v8::FunctionTemplate;
using v8::HandleScope;
using v8::Isolate;
using v8::Local;
using v8::Object;
using v8::String;
using v8::Value;

namespace {

void Hello(const FunctionCallbackInfo<Value>& info) {
  Isolate* isolate = info.GetIsolate();
  Local<String> result = String::NewFromUtf8(isolate, "hello from v8").ToLocalChecked();
  info.GetReturnValue().Set(result);
}

void Init(Local<Object> exports, Local<Value> module, Local<Context> context) {
  (void)module;
  (void)context;
  NODE_SET_METHOD(exports, "hello", Hello);
}

}  // namespace

NODE_MODULE_CONTEXT_AWARE(v8_hello, Init)
