import RasterPluginSDK
import Foundation

public enum FilesystemPlugin {
    public static func register() {
        RasterPlugin.register(plugin: "Filesystem", method: "getCacheDirectory") { call in
            let url = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask).first
            guard let url else {
                call.replyErr(code: "UNAVAILABLE", message: "Cache directory unavailable")
                return
            }
            call.replyOk(["uri": url.absoluteString])
        }

        RasterPlugin.register(plugin: "Filesystem", method: "readText") { call in
            guard let uri = call.args?["uri"] as? String, let url = URL(string: uri) else {
                call.replyErr(code: "INVALID_ARGS", message: "uri is required")
                return
            }
            do {
                let text = try String(contentsOf: url, encoding: .utf8)
                call.replyOk(["text": text])
            } catch {
                call.replyErr(code: "READ_FAILED", message: error.localizedDescription)
            }
        }

        RasterPlugin.register(plugin: "Filesystem", method: "writeText") { call in
            guard let uri = call.args?["uri"] as? String,
                  let text = call.args?["text"] as? String,
                  let url = URL(string: uri) else {
                call.replyErr(code: "INVALID_ARGS", message: "uri and text are required")
                return
            }
            do {
                try text.write(to: url, atomically: true, encoding: .utf8)
                call.replyOk(["uri": url.absoluteString])
            } catch {
                call.replyErr(code: "WRITE_FAILED", message: error.localizedDescription)
            }
        }
    }
}