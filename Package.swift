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
            url: "https://github.com/Ray-D-Song/raster/releases/download/v0.1.0-alpha.13/RasterRuntime.xcframework.zip",
            checksum: "9f3f5da5a224012a53f3b2a5724b02c56abb9ce49aad6675ae2b9410bd9308e3"
        )
    ]
)
