// swift-tools-version: 6.1

import PackageDescription

let package = Package(
    name: "MacCommandBar",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(name: "MacCommandBar", targets: ["MacCommandBar"]),
        .library(name: "MacCommandBarKit", targets: ["MacCommandBarKit"])
    ],
    targets: [
        .executableTarget(
            name: "MacCommandBar",
            dependencies: ["MacCommandBarKit"]
        ),
        .target(
            name: "MacCommandBarKit"
        ),
        .testTarget(
            name: "MacCommandBarKitTests",
            dependencies: ["MacCommandBarKit"]
        )
    ]
)

