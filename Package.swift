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
            checksum: "e195ef92b8427baa0d8ffc4033ff7cb4e71c15d10cb96a0c55293064d7f60d36"
        )
    ]
)
