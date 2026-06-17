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
        )
    ],
    targets: [
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
