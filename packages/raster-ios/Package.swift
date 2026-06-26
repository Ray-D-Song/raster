// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "RasterIOSLocal",
    platforms: [
        .iOS(.v15)
    ],
    products: [
        .library(
            name: "RasterIOS",
            targets: ["RasterIOS"]
        ),
        .library(
            name: "RasterPluginSDK",
            targets: ["RasterPluginSDK"]
        )
    ],
    targets: [
        .target(
            name: "RasterPluginSDK",
            dependencies: ["RasterRuntime"],
            path: "Sources/RasterPluginSDK"
        ),
        .target(
            name: "RasterIOS",
            dependencies: ["RasterRuntime"],
            path: "Sources/RasterIOS"
        ),
        .binaryTarget(
            name: "RasterRuntime",
            path: "dist/RasterRuntime.xcframework"
        )
    ]
)
