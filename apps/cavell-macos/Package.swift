// swift-tools-version: 5.9
import PackageDescription

let package = Package(
  name: "CavellMacOS",
  platforms: [
    .macOS(.v12),
  ],
  products: [
    .executable(
      name: "CavellApp",
      targets: ["CavellApp"]
    ),
  ],
  targets: [
    .executableTarget(
      name: "CavellApp",
      path: "Sources/CavellApp"
    ),
  ]
)
