#include "v8_bridge_helpers.h"

#include <v8-context.h>

namespace v8 {

namespace {
thread_local int g_context_depth = 0;
}  // namespace

void Context::Enter() {
  g_context_depth++;
}

void Context::Exit() {
  if (g_context_depth > 0) {
    g_context_depth--;
  }
}

Isolate* Context::GetIsolate() {
  return Isolate::GetCurrent();
}

}  // namespace v8
