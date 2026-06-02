import Foundation

struct DistributionPackageMetadata: Equatable {
  let schemaVersion: Int
  let signing: String
  let distributionTrust: String
  let architecture: String
  let minimumSystemVersion: String
  let modelDelivery: String
  let modelWeightsBundled: Bool
  let pithAccountRequired: Bool
  let defaultLocalExecutionSafetyMode: String
  let localExecutionSafetyModes: [String]
  let maxAppBundleBytes: Int
  let maxZipArtifactBytes: Int
  let sandboxMode: String
  let sandboxBackend: String
  let sandboxFallback: String
  let sandboxNetworkDefault: String
  let dailyDriverStageSource: String
  let dailyDriverNextActionSource: String
  let dailyDriverPresentation: String
  let firstAppOpenActionContract: String
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
    let sizeBudget = dictionary(manifest, "sizeBudget", fallback: [:])

    return DistributionPackageMetadata(
      schemaVersion: int(manifest, "schemaVersion", fallback: 1),
      signing: string(manifest, "signing", fallback: "development"),
      distributionTrust: string(
        manifest,
        "distributionTrust",
        fallback: distributionTrustFallback(
          signing: string(manifest, "signing", fallback: "development")
        )
      ),
      architecture: string(manifest, "architecture", fallback: "unknown"),
      minimumSystemVersion: string(manifest, "minimumSystemVersion", fallback: "12.0"),
      modelDelivery: string(manifest, "modelDelivery", fallback: "in-app-download"),
      modelWeightsBundled: bool(manifest, "modelWeightsBundled", fallback: false),
      pithAccountRequired: bool(manifest, "pithAccountRequired", fallback: false),
      defaultLocalExecutionSafetyMode: string(
        manifest,
        "defaultLocalExecutionSafetyMode",
        fallback: "askBeforeChange"
      ),
      localExecutionSafetyModes: stringArray(
        manifest,
        "localExecutionSafetyModes",
        fallback: ["explore", "askBeforeChange", "approvedWorkspaceExecution"]
      ),
      maxAppBundleBytes: int(sizeBudget, "maxAppBundleBytes", fallback: 0),
      maxZipArtifactBytes: int(sizeBudget, "maxZipArtifactBytes", fallback: 0),
      sandboxMode: string(manifest, "sandboxMode", fallback: "workspaceReadWrite"),
      sandboxBackend: string(manifest, "sandboxBackend", fallback: "runtime-detected"),
      sandboxFallback: string(
        manifest,
        "sandboxFallback",
        fallback: "processOnlyWhenNativeUnavailable"
      ),
      sandboxNetworkDefault: string(manifest, "sandboxNetworkDefault", fallback: "disabled"),
      dailyDriverStageSource: string(
        manifest,
        "dailyDriverStageSource",
        fallback: "runtime/readiness"
      ),
      dailyDriverNextActionSource: string(
        manifest,
        "dailyDriverNextActionSource",
        fallback: "runtime/readiness"
      ),
      dailyDriverPresentation: string(
        manifest,
        "dailyDriverPresentation",
        fallback: "app-header-inspector"
      ),
      firstAppOpenActionContract: string(
        manifest,
        "firstAppOpenActionContract",
        fallback: FirstRequestPromptPresenter.firstAppOpenActionContractID
      ),
      sourceCommit: string(manifest, "sourceCommit", fallback: "development")
    )
  }

  static let development = DistributionPackageMetadata(
    schemaVersion: 0,
    signing: "development",
    distributionTrust: "development",
    architecture: "unknown",
    minimumSystemVersion: "12.0",
    modelDelivery: "in-app-download",
    modelWeightsBundled: false,
    pithAccountRequired: false,
    defaultLocalExecutionSafetyMode: "askBeforeChange",
    localExecutionSafetyModes: [
      "explore",
      "askBeforeChange",
      "approvedWorkspaceExecution",
    ],
    maxAppBundleBytes: 0,
    maxZipArtifactBytes: 0,
    sandboxMode: "workspaceReadWrite",
    sandboxBackend: "runtime-detected",
    sandboxFallback: "processOnlyWhenNativeUnavailable",
    sandboxNetworkDefault: "disabled",
    dailyDriverStageSource: "runtime/readiness",
    dailyDriverNextActionSource: "runtime/readiness",
    dailyDriverPresentation: "app-header-inspector",
    firstAppOpenActionContract: FirstRequestPromptPresenter.firstAppOpenActionContractID,
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

  private static func dictionary(
    _ manifest: [String: Any],
    _ key: String,
    fallback: [String: Any]
  ) -> [String: Any] {
    guard let value = manifest[key] as? [String: Any] else {
      return fallback
    }
    return value
  }

  private static func stringArray(
    _ manifest: [String: Any],
    _ key: String,
    fallback: [String]
  ) -> [String] {
    guard let value = manifest[key] as? [String] else {
      return fallback
    }
    return value
  }

  private static func distributionTrustFallback(signing: String) -> String {
    switch signing {
    case "developer-id":
      return "developer-id-signed-notarized"
    case "ad-hoc":
      return "ad-hoc-not-notarized"
    case "unsigned":
      return "unsigned-local-build"
    default:
      return "development"
    }
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
    let sandbox = sandboxSummary(metadata)
    let dailyDriver = dailyDriverSummary(metadata)
    let firstOpen = firstAppOpenSummary(metadata)
    let packageSize = packageSizeSummary(metadata)
    let identity = identitySummary(metadata)
    let execution = localExecutionSummary(metadata)
    let source = sourceSummary(metadata.sourceCommit)
    let releaseProof = "\(identity); \(modelDelivery); \(weightPolicy); \(execution); \(packageSize); \(sandbox); \(dailyDriver); \(firstOpen); \(source)."

    switch metadata.distributionTrust {
    case "developer-id-signed-notarized":
      return DistributionTrustSummary(
        title: "Trusted Installer",
        summary: "Developer ID signed and notarized for \(platform).",
        detail: "Install from the DMG, launch normally, then choose one verified local model. \(releaseProof)",
        setupDetail: nil
      )
    case "ad-hoc-not-notarized":
      return DistributionTrustSummary(
        title: "Untrusted Ad-Hoc Build",
        summary: "Ad-hoc signed and not notarized for \(platform).",
        detail: "If macOS blocks first launch, use Privacy & Security > Open Anyway or Control-click Pith.app and choose Open. \(releaseProof)",
        setupDetail: "Installer trust: if macOS blocked first launch, use Privacy & Security > Open Anyway or Control-click Pith.app and choose Open."
      )
    case "unsigned-local-build":
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

  private static func identitySummary(_ metadata: DistributionPackageMetadata) -> String {
    metadata.pithAccountRequired ? "Pith account required" : "no Pith account required"
  }

  private static func localExecutionSummary(_ metadata: DistributionPackageMetadata) -> String {
    let defaultMode = LocalExecutionSafetyModePresenter.detailed(
      metadata.defaultLocalExecutionSafetyMode
    )
    let modes = metadata.localExecutionSafetyModes
      .map { LocalExecutionSafetyModePresenter.detailed($0) }
      .joined(separator: ", ")
    return "local execution mode: \(defaultMode); modes: \(modes)"
  }

  private static func sandboxSummary(_ metadata: DistributionPackageMetadata) -> String {
    guard metadata.sandboxMode == "workspaceReadWrite" else {
      return "sandbox mode: \(metadata.sandboxMode)"
    }
    let fallback = metadata.sandboxFallback == "processOnlyWhenNativeUnavailable"
      ? "process-only fallback is shown when native sandbox is unavailable"
      : "fallback: \(metadata.sandboxFallback)"
    let network = metadata.sandboxNetworkDefault == "disabled"
      ? "network off by default"
      : "network default: \(metadata.sandboxNetworkDefault)"
    return "workspace sandbox checks run at runtime; \(fallback); \(network)"
  }

  private static func packageSizeSummary(_ metadata: DistributionPackageMetadata) -> String {
    guard metadata.maxAppBundleBytes > 0, metadata.maxZipArtifactBytes > 0 else {
      return "package size budget: unavailable"
    }
    let appBudget = mebibytes(metadata.maxAppBundleBytes)
    let installerBudget = mebibytes(metadata.maxZipArtifactBytes)
    return "package size budget: app <= \(appBudget), installer artifact <= \(installerBudget)"
  }

  private static func mebibytes(_ bytes: Int) -> String {
    let oneMebibyte = 1024 * 1024
    if bytes % oneMebibyte == 0 {
      return "\(bytes / oneMebibyte) MiB"
    }
    return String(format: "%.1f MiB", Double(bytes) / Double(oneMebibyte))
  }

  private static func dailyDriverSummary(_ metadata: DistributionPackageMetadata) -> String {
    guard metadata.dailyDriverStageSource == "runtime/readiness",
          metadata.dailyDriverNextActionSource == "runtime/readiness"
    else {
      return "daily driver readiness: custom"
    }
    let presentation = metadata.dailyDriverPresentation == "app-header-inspector"
      ? "shown in app header and inspector"
      : "presentation: \(metadata.dailyDriverPresentation)"
    return "daily-driver next action comes from runtime readiness and is \(presentation)"
  }

  private static func firstAppOpenSummary(_ metadata: DistributionPackageMetadata) -> String {
    guard metadata.firstAppOpenActionContract == FirstRequestPromptPresenter.firstAppOpenActionContractID else {
      return "first app-open action: \(metadata.firstAppOpenActionContract)"
    }
    return FirstRequestPromptPresenter.firstAppOpenActionTrustSummary()
  }
}
