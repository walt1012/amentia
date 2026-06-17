// swift-tools-version: 5.9
import PackageDescription

let package = Package(
  name: "AmentiaMacOS",
  platforms: [
    .macOS(.v12),
  ],
  products: [
    .executable(
      name: "AmentiaApp",
      targets: ["AmentiaApp"]
    ),
  ],
  targets: [
    .executableTarget(
      name: "AmentiaApp",
      path: "Sources/AmentiaApp"
    ),
    .testTarget(
      name: "AmentiaAppTests",
      dependencies: ["AmentiaApp"],
      path: "Tests/AmentiaAppTests"
    ),
  ]
)
