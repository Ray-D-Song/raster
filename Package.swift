// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "Raster",
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
            path: "packages/raster-ios/Sources/RasterIOS"
        ),
        .binaryTarget(
            name: "RasterRuntime",
            url: "https://github.com/Ray-D-Song/raster/releases/download/v0.1.0-alpha.16/RasterRuntime.xcframework.zip",
            checksum: "22e5596f5a1c5da939b6b2c72d4db16887e4dc5af54e8123bb9b4585dbcfb1b7"
        )
    ]
)
