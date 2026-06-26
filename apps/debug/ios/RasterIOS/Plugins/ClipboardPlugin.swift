import RasterPluginSDK
import UIKit

public enum ClipboardPlugin {
    public static func register() {
        RasterPlugin.register(plugin: "Clipboard", method: "getString") { call in
            let value = UIPasteboard.general.string
            call.replyOk(["value": value as Any])
        }

        RasterPlugin.register(plugin: "Clipboard", method: "setString") { call in
            guard let value = call.args?["value"] as? String else {
                call.replyErr(code: "INVALID_ARGS", message: "value is required")
                return
            }
            UIPasteboard.general.string = value
            call.replyOk(["ok": true])
        }
    }
}