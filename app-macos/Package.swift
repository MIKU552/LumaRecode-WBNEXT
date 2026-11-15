// swift-tools-version: 6.0
// LumaRecode for macOS.
//
// The Rust core is built by scripts/build-macos.sh into Libraries/ before
// swift build runs; the package only declares the link requirement. Building
// through Xcode (open Package.swift) needs the same staged library, which the
// script produces with --stage-only.
import PackageDescription

let package = Package(
    name: "LumaRecode",
    platforms: [
        // First release targets macOS 27 on Apple Silicon only.
        .macOS("27.0")
    ],
    products: [
        .executable(name: "LumaRecode", targets: ["LumaRecodeApp"]),
        .library(name: "LumaKit", targets: ["LumaKit"]),
    ],
    targets: [
        // C ABI declarations; the header shim resolves to native/include.
        .systemLibrary(name: "CLumaCore", path: "Sources/CLumaCore"),
        .target(
            name: "LumaKit",
            dependencies: ["CLumaCore"],
            path: "Sources/LumaKit"
        ),
        .executableTarget(
            name: "LumaRecodeApp",
            dependencies: ["LumaKit"],
            path: "Sources/LumaRecodeApp",
            linkerSettings: [.linkedLibrary("lumacore")]
        ),
        .testTarget(
            name: "LumaKitTests",
            dependencies: ["LumaKit"],
            path: "Tests/LumaKitTests",
            linkerSettings: [.linkedLibrary("lumacore")]
        ),
    ]
)
