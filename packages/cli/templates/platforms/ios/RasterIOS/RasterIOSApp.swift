import SwiftUI
import RasterIOS

@main
struct RasterIOSApp: App {
    var body: some Scene {
        WindowGroup {
            RasterAppView(configuration: .default)
                .ignoresSafeArea()
        }
    }
}
