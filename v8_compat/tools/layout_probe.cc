#include <cstdio>
#include <v8-internal.h>
#include <v8-local-handle.h>

int main() {
  using I = v8::internal::Internals;
  printf("sizeof(HandleScope)=%zu\n", sizeof(v8::HandleScope));
  printf("sizeof(EscapableHandleScope)=%zu\n", sizeof(v8::EscapableHandleScope));
  printf("kIsolateHandleScopeDataOffset=%d\n", I::kIsolateHandleScopeDataOffset);
  printf("kIsolateRootsOffset=%d\n", I::kIsolateRootsOffset);
  printf("kHandleScopeDataSize=%d\n", I::kHandleScopeDataSize);
  return 0;
}
