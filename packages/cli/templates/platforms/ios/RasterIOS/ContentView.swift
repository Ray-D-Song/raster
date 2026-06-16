import SwiftUI
import RasterIOS

struct ContentView: View {
    var body: some View {
        RasterAppView(configuration: .default)
            .ignoresSafeArea()
    }
}

#Preview {
    ContentView()
}
