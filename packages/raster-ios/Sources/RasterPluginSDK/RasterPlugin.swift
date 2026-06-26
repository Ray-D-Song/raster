import Foundation
import UIKit
import RasterRuntime

public enum RasterPlugin {
    public struct Call {
        public let id: UInt64
        public let args: [String: Any]?

        fileprivate let argsJson: String

        public init(id: UInt64, argsJson: String) {
            self.id = id
            self.argsJson = argsJson
            if let data = argsJson.data(using: .utf8),
               let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
                self.args = object
            } else {
                self.args = nil
            }
        }

        public func replyOk(_ result: [String: Any]) {
            guard let data = try? JSONSerialization.data(withJSONObject: result),
                  let json = String(data: data, encoding: .utf8) else {
                replyErr(code: "INVALID_RESULT", message: "Failed to encode result JSON")
                return
            }
            json.withCString { ptr in
                raster_plugin_reply_ok(id, ptr)
            }
        }

        public func replyErr(code: String, message: String) {
            code.withCString { codePtr in
                message.withCString { messagePtr in
                    raster_plugin_reply_err(id, codePtr, messagePtr)
                }
            }
        }
    }

    public static func register(
        plugin: String,
        method: String,
        handler: @escaping (Call) -> Void
    ) {
        let context = Unmanaged.passRetained(PluginHandlerBox(handler: handler)).toOpaque()
        plugin.withCString { pluginPtr in
            method.withCString { methodPtr in
                _ = raster_plugin_register_method(pluginPtr, methodPtr, pluginHandlerTrampoline, context)
            }
        }
    }

    public static func rootViewController() -> UIViewController? {
        guard let pointer = raster_ios_host_view_controller() else {
            return nil
        }
        return Unmanaged<UIViewController>.fromOpaque(pointer).takeUnretainedValue()
    }

    public static func emit(plugin: String, event: String, data: [String: Any] = [:]) {
        guard let payload = try? JSONSerialization.data(withJSONObject: data),
              let json = String(data: payload, encoding: .utf8) else {
            return
        }
        plugin.withCString { pluginPtr in
            event.withCString { eventPtr in
                json.withCString { jsonPtr in
                    raster_plugin_emit_event(pluginPtr, eventPtr, jsonPtr)
                }
            }
        }
    }
}

private final class PluginHandlerBox {
    let handler: (RasterPlugin.Call) -> Void

    init(handler: @escaping (RasterPlugin.Call) -> Void) {
        self.handler = handler
    }
}

private func pluginHandlerTrampoline(call: UnsafeRawPointer?) {
    guard let call else { return }
    let typedCall = call.assumingMemoryBound(to: RasterPluginCall.self)
    let argsJson = String(cString: typedCall.pointee.args_json)
    let pluginCall = RasterPlugin.Call(id: typedCall.pointee.call_id, argsJson: argsJson)
    guard let context = typedCall.pointee.context else { return }
    let box = Unmanaged<PluginHandlerBox>.fromOpaque(context).takeUnretainedValue()
    box.handler(pluginCall)
}