import SwiftUI
import RasterIOS
import RasterPluginSDK

@main
struct RasterIOSApp: App {
    init() {
        RasterPlugins.registerAll()
    }

    var body: some Scene {
        WindowGroup {
            RasterAppView(configuration: .default)
                .ignoresSafeArea()
        }
    }
}
