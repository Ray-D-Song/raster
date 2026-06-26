import RasterPluginSDK
import UIKit

public enum HapticsPlugin {
    public static func register() {
        RasterPlugin.register(plugin: "Haptics", method: "impact") { call in
            DispatchQueue.main.async {
                let style = (call.args?["style"] as? String) ?? "medium"
                let generator: UIImpactFeedbackGenerator
                switch style {
                case "light":
                    generator = UIImpactFeedbackGenerator(style: .light)
                case "heavy":
                    generator = UIImpactFeedbackGenerator(style: .heavy)
                default:
                    generator = UIImpactFeedbackGenerator(style: .medium)
                }
                generator.prepare()
                generator.impactOccurred()
                call.replyOk(["ok": true])
            }
        }

        RasterPlugin.register(plugin: "Haptics", method: "vibrate") { call in
            DispatchQueue.main.async {
                let generator = UINotificationFeedbackGenerator()
                generator.prepare()
                generator.notificationOccurred(.success)
                call.replyOk(["ok": true])
            }
        }
    }
}