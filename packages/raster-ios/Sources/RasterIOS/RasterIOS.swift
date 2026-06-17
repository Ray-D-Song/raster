import Foundation
import SwiftUI
import UIKit
import RasterRuntime

public enum RasterBundleMode: Equatable {
    case development
    case production
}

public struct RasterConfiguration: Equatable {
    public var mode: RasterBundleMode
    public var bundleName: String
    public var bundleURL: URL?
    public var devConfigURL: URL?

    public init(mode: RasterBundleMode, bundleName: String = "raster/app.js", bundleURL: URL? = nil, devConfigURL: URL? = nil) {
        self.mode = mode
        self.bundleName = bundleName
        self.bundleURL = bundleURL
        self.devConfigURL = devConfigURL
    }

    public init(bundleName: String = "raster/app.js", bundleURL: URL, devConfigURL: URL? = nil) {
        self.mode = .production
        self.bundleName = bundleName
        self.bundleURL = bundleURL
        self.devConfigURL = devConfigURL
    }

    public static var `default`: RasterConfiguration {
        let bundle = Bundle.main
        #if DEBUG
        return RasterConfiguration(
            mode: .development,
            bundleName: "raster/app.js",
            devConfigURL: bundle.url(forResource: "dev", withExtension: "json", subdirectory: "raster")
        )
        #else
        return RasterConfiguration(
            mode: .production,
            bundleName: "raster/app.js",
            bundleURL: bundle.url(forResource: "app", withExtension: "js", subdirectory: "raster")
        )
        #endif
    }
}

public final class RasterDevServer {
    public static let port = 14201
}

public struct RasterAppView: UIViewRepresentable {
    private let configuration: RasterConfiguration

    public init(configuration: RasterConfiguration = .default) {
        self.configuration = configuration
    }

    public func makeCoordinator() -> Coordinator {
        Coordinator(configuration: configuration)
    }

    public func makeUIView(context: Context) -> UIView {
        let view = UIView(frame: .zero)
        view.backgroundColor = .clear
        context.coordinator.start()
        return view
    }

    public func updateUIView(_ uiView: UIView, context: Context) {
        context.coordinator.update(configuration: configuration)
    }

    public static func dismantleUIView(_ uiView: UIView, coordinator: Coordinator) {
        coordinator.stop()
    }

    public final class Coordinator {
        private var configuration: RasterConfiguration
        private var displayLink: CADisplayLink?
        private var observers: [NSObjectProtocol] = []
        private var started = false

        init(configuration: RasterConfiguration) {
            self.configuration = configuration
        }

        func update(configuration: RasterConfiguration) {
            self.configuration = configuration
        }

        func start() {
            guard !started else { return }
            started = true

            do {
                let ok: Bool
                switch configuration.mode {
                case .development:
                    guard let devConfigURL = configuration.devConfigURL else {
                        assertionFailure("Missing Raster iOS dev config: raster/dev.json")
                        return
                    }
                    let devConfig = try String(contentsOf: devConfigURL, encoding: .utf8)
                    ok = configuration.bundleName.withCString { namePtr in
                        devConfig.withCString { devPtr in
                            raster_ios_run_app(namePtr, nil, devPtr)
                        }
                    }
                case .production:
                    guard let bundleURL = configuration.bundleURL else {
                        assertionFailure("Missing Raster iOS bundle: raster/app.js")
                        return
                    }
                    let source = try String(contentsOf: bundleURL, encoding: .utf8)
                    ok = source.withCString { sourcePtr in
                        configuration.bundleName.withCString { namePtr in
                            raster_ios_run_app(namePtr, sourcePtr, nil)
                        }
                    }
                }
                if !ok {
                    let message = raster_ios_last_error().map { String(cString: $0) } ?? "unknown Raster iOS runtime error"
                    assertionFailure(message)
                    return
                }
            } catch {
                assertionFailure("Failed to load Raster iOS bundle: \(error)")
                return
            }

            installLifecycleObservers()
            startDisplayLink()
        }

        func stop() {
            displayLink?.invalidate()
            displayLink = nil
            for observer in observers {
                NotificationCenter.default.removeObserver(observer)
            }
            observers.removeAll()
            raster_ios_will_terminate()
        }

        private func startDisplayLink() {
            guard displayLink == nil else { return }
            let link = CADisplayLink(target: self, selector: #selector(renderFrame))
            link.add(to: .main, forMode: .common)
            displayLink = link
        }

        private func installLifecycleObservers() {
            let center = NotificationCenter.default
            observers = [
                center.addObserver(forName: UIApplication.willEnterForegroundNotification, object: nil, queue: .main) { [weak self] _ in
                    DispatchQueue.main.async {
                        guard let self, self.started else { return }
                        raster_ios_will_enter_foreground()
                        self.startDisplayLink()
                    }
                },
                center.addObserver(forName: UIApplication.didBecomeActiveNotification, object: nil, queue: .main) { [weak self] _ in
                    DispatchQueue.main.async {
                        guard let self, self.started else { return }
                        raster_ios_did_become_active()
                    }
                },
                center.addObserver(forName: UIApplication.willResignActiveNotification, object: nil, queue: .main) { [weak self] _ in
                    DispatchQueue.main.async {
                        guard let self, self.started else { return }
                        raster_ios_will_resign_active()
                    }
                },
                center.addObserver(forName: UIApplication.didEnterBackgroundNotification, object: nil, queue: .main) { [weak self] _ in
                    DispatchQueue.main.async {
                        guard let self, self.started else { return }
                        raster_ios_did_enter_background()
                        self.displayLink?.invalidate()
                        self.displayLink = nil
                    }
                }
            ]
        }

        @objc private func renderFrame() {
            raster_ios_request_frame()
        }
    }
}
