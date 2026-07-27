#include "v8_bridge_helpers.h"

#include <v8.h>

namespace v8 {

Local<Signature> Signature::New(Isolate* isolate, Local<FunctionTemplate> receiver) {
  (void)isolate;
  (void)receiver;
  return Local<Signature>();
}

}  // namespace v8
