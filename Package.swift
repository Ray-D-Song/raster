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
            checksum: "87ee709d11f0f69c4b501162733b5ccfed2b61833e83d42e7d66a047e73ee4fe"
        )
    ]
)
