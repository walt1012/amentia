// swift-tools-version: 5.9
import PackageDescription

let package = Package(
  name: "PithMacOS",
  platforms: [
    .macOS(.v12),
  ],
  products: [
    .executable(
      name: "PithApp",
      targets: ["PithApp"]
    ),
  ],
  targets: [
    .executableTarget(
      name: "PithApp",
      path: "Sources/PithApp"
    ),
  ]
)
