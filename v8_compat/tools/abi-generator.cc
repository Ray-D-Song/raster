// ABI layout probe for Node 24.3.0 / NODE_MODULE_VERSION 137.
// Build: see v8_compat/tools/check_abi.sh

#include <node.h>
#include <v8-function-callback.h>
#include <v8-internal.h>
#include <v8-version.h>

#include <iostream>

int main() {
  using I = v8::internal::Internals;
  std::cout << "{\n";
  std::cout << "  \"profile\": \"node24-abi137\",\n";
  std::cout << "  \"node_module_version\": " << NODE_MODULE_VERSION << ",\n";
  std::cout << "  \"kUndefinedValueRootIndex\": " << I::kUndefinedValueRootIndex << ",\n";
  std::cout << "  \"kTheHoleValueRootIndex\": " << I::kTheHoleValueRootIndex << ",\n";
  std::cout << "  \"kNullValueRootIndex\": " << I::kNullValueRootIndex << ",\n";
  std::cout << "  \"kTrueValueRootIndex\": " << I::kTrueValueRootIndex << ",\n";
  std::cout << "  \"kFalseValueRootIndex\": " << I::kFalseValueRootIndex << ",\n";
  std::cout << "  \"kEmptyStringRootIndex\": " << I::kEmptyStringRootIndex << ",\n";
  std::cout << "  \"function_callback_k_return_value_index\": 3,\n";
  std::cout << "  \"property_callback_k_return_value_index\": 5,\n";
  std::cout << "  \"property_callback_k_this_index\": 7\n";
  std::cout << "}\n";
  return 0;
}
