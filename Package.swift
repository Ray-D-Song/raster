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
            url: "https://github.com/Ray-D-Song/raster/releases/download/v0.1.0-alpha.12/RasterRuntime.xcframework.zip",
            checksum: "1d08d244ced08ae50860dfa22accc1cb03d985ba1df3608670625471da597177"
        )
    ]
)
