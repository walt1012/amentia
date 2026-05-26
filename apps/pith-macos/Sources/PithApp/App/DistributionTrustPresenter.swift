import Foundation

struct DistributionPackageMetadata: Equatable {
  let schemaVersion: Int
  let signing: String
  let architecture: String
  let minimumSystemVersion: String
  let modelDelivery: String
  let modelWeightsBundled: Bool
  let sourceCommit: String

  static let current = load()

  static func load(bundle: Bundle = .main) -> DistributionPackageMetadata {
    guard let url = bundle.url(forResource: "PithPackage", withExtension: "json"),
          let data = try? Data(contentsOf: url),
          let metadata = fromManifestData(data)
    else {
      return development
    }

    return metadata
  }

  static func fromManifestData(_ data: Data) -> DistributionPackageMetadata? {
    guard let object = try? JSONSerialization.jsonObject(with: data),
          let manifest = object as? [String: Any]
    else {
      return nil
    }
    guard int(manifest, "schemaVersion", fallback: 0) == 1 else {
      return nil
    }

    return DistributionPackageMetadata(
      schemaVersion: int(manifest, "schemaVersion", fallback: 1),
      signing: string(manifest, "signing", fallback: "development"),
      architecture: string(manifest, "architecture", fallback: "unknown"),
      minimumSystemVersion: string(manifest, "minimumSystemVersion", fallback: "12.0"),
      modelDelivery: string(manifest, "modelDelivery", fallback: "in-app-download"),
      modelWeightsBundled: bool(manifest, "modelWeightsBundled", fallback: false),
      sourceCommit: string(manifest, "sourceCommit", fallback: "development")
    )
  }

  static let development = DistributionPackageMetadata(
    schemaVersion: 0,
    signing: "development",
    architecture: "unknown",
    minimumSystemVersion: "12.0",
    modelDelivery: "in-app-download",
    modelWeightsBundled: false,
    sourceCommit: "development"
  )

  private static func string(
    _ manifest: [String: Any],
    _ key: String,
    fallback: String
  ) -> String {
    guard let value = manifest[key] as? String else {
      return fallback
    }
    return value
  }

  private static func bool(
    _ manifest: [String: Any],
    _ key: String,
    fallback: Bool
  ) -> Bool {
    guard let value = manifest[key] as? Bool else {
      return fallback
    }
    return value
  }

  private static func int(
    _ manifest: [String: Any],
    _ key: String,
    fallback: Int
  ) -> Int {
    guard let value = manifest[key] as? Int else {
      return fallback
    }
    return value
  }
}

struct DistributionTrustSummary: Equatable {
  let title: String
  let summary: String
  let detail: String
  let setupDetail: String?
}

enum DistributionTrustPresenter {
  static func summary(
    _ metadata: DistributionPackageMetadata = .current
  ) -> DistributionTrustSummary {
    let modelDelivery = metadata.modelDelivery == "in-app-download"
      ? "models download in app"
      : "model delivery: \(metadata.modelDelivery)"
    let weightPolicy = metadata.modelWeightsBundled
      ? "model weights bundled"
      : "model weights are not bundled"
    let platform = "macOS \(metadata.minimumSystemVersion)+ \(metadata.architecture)"
    let source = sourceSummary(metadata.sourceCommit)

    switch metadata.signing {
    case "developer-id":
      return DistributionTrustSummary(
        title: "Trusted Installer",
        summary: "Developer ID signed and notarized for \(platform).",
        detail: "Install from the DMG, launch normally, then choose one verified local model. \(modelDelivery); \(weightPolicy); \(source).",
        setupDetail: nil
      )
    case "ad-hoc":
      return DistributionTrustSummary(
        title: "Untrusted Ad-Hoc Build",
        summary: "Ad-hoc signed and not notarized for \(platform).",
        detail: "If macOS blocks first launch, use Privacy & Security > Open Anyway or Control-click Pith.app and choose Open. \(modelDelivery); \(weightPolicy); \(source).",
        setupDetail: "Installer trust: if macOS blocked first launch, use Privacy & Security > Open Anyway or Control-click Pith.app and choose Open."
      )
    case "unsigned":
      return DistributionTrustSummary(
        title: "Unsigned Build",
        summary: "Unsigned local build for \(platform).",
        detail: "Use this only for development or explicit testing. Public users should prefer Developer ID builds or clearly marked ad-hoc prereleases. \(source).",
        setupDetail: "Installer trust: this is an unsigned build, so macOS may require manual approval before first launch."
      )
    default:
      return DistributionTrustSummary(
        title: "Development Build",
        summary: "Package metadata is unavailable in this run.",
        detail: "Release DMGs include PithPackage.json, README-FIRST.txt, SHA-256 checksum, and a release manifest.",
        setupDetail: nil
      )
    }
  }

  private static func sourceSummary(_ sourceCommit: String) -> String {
    guard sourceCommit.count >= 12, sourceCommit != "development" else {
      return "source: development"
    }
    return "source: \(sourceCommit.prefix(12))"
  }
}
