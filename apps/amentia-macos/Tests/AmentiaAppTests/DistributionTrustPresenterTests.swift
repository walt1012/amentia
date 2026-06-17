@testable import AmentiaApp
import XCTest

final class DistributionTrustPresenterTests: XCTestCase {
  func testAdHocBuildExplainsGatekeeperPath() throws {
    let summary = DistributionTrustPresenter.summary(
      try metadata(signing: "ad-hoc")
    )

    XCTAssertEqual(summary.title, "Manual Open Required")
    XCTAssertTrue(summary.summary.contains("one extra macOS approval"))
    XCTAssertTrue(summary.detail.contains("Open Anyway"))
    XCTAssertTrue(summary.detail.contains("setup continues in app"))
    XCTAssertFalse(summary.detail.contains("package size budget"))
    XCTAssertFalse(summary.detail.contains("source:"))
    XCTAssertTrue(summary.advancedDetail.contains("model weights are not bundled"))
    XCTAssertTrue(summary.advancedDetail.contains("no Amentia account required"))
    XCTAssertTrue(summary.advancedDetail.contains("action safety mode: ask-before-change"))
    XCTAssertTrue(summary.advancedDetail.contains("package size budget"))
    XCTAssertTrue(summary.advancedDetail.contains("app <= 250 MiB"))
    XCTAssertTrue(summary.advancedDetail.contains("installer artifact <= 150 MiB"))
    XCTAssertTrue(summary.advancedDetail.contains("process-only fallback"))
    XCTAssertTrue(summary.advancedDetail.contains("daily-driver next action"))
    XCTAssertTrue(summary.advancedDetail.contains("Amentia status"))
    XCTAssertFalse(summary.advancedDetail.contains("runtime readiness"))
    XCTAssertTrue(summary.advancedDetail.contains("Understand Project"))
    XCTAssertTrue(summary.advancedDetail.contains("Pick Next Step"))
    XCTAssertTrue(summary.advancedDetail.contains("short cowork prompt"))
    XCTAssertTrue(summary.advancedDetail.contains("source: 0123456789ab"))
    XCTAssertTrue(summary.setupDetail?.contains("Control-click Amentia.app") == true)
  }

  func testDeveloperIDBuildAvoidsManualGatekeeperCopy() throws {
    let summary = DistributionTrustPresenter.summary(
      try metadata(signing: "developer-id", distributionTrust: "developer-id-signed-notarized")
    )

    XCTAssertEqual(summary.title, "Verified Installer")
    XCTAssertTrue(summary.summary.contains("Signed and notarized"))
    XCTAssertNil(summary.setupDetail)
    XCTAssertFalse(summary.detail.contains("Open Anyway"))
    XCTAssertTrue(summary.advancedDetail.contains("Developer ID signed and notarized"))
  }

  func testDevelopmentFallbackNamesReleaseAssets() {
    let summary = DistributionTrustPresenter.summary(.development)

    XCTAssertEqual(summary.title, "Development Run")
    XCTAssertTrue(summary.detail.contains("GitHub Release DMG"))
    XCTAssertTrue(summary.advancedDetail.contains("README-FIRST.txt"))
    XCTAssertTrue(summary.advancedDetail.contains("app package metadata"))
    XCTAssertTrue(summary.advancedDetail.contains("release manifest"))
    XCTAssertFalse(summary.advancedDetail.contains("AmentiaPackage"))
  }

  func testManifestParsingKeepsPackageContract() throws {
    let parsed = try XCTUnwrap(
      DistributionPackageMetadata.fromManifestData(manifestData(signing: "ad-hoc"))
    )

    XCTAssertEqual(parsed.signing, "ad-hoc")
    XCTAssertEqual(parsed.distributionTrust, "ad-hoc-not-notarized")
    XCTAssertEqual(parsed.schemaVersion, 1)
    XCTAssertEqual(parsed.architecture, "x86_64")
    XCTAssertEqual(parsed.minimumSystemVersion, "12.0")
    XCTAssertEqual(parsed.modelDelivery, "in-app-download")
    XCTAssertFalse(parsed.modelWeightsBundled)
    XCTAssertFalse(parsed.amentiaAccountRequired)
    XCTAssertEqual(parsed.defaultLocalExecutionSafetyMode, "askBeforeChange")
    XCTAssertEqual(
      parsed.localExecutionSafetyModes,
      ["explore", "askBeforeChange", "approvedWorkspaceExecution"]
    )
    XCTAssertEqual(parsed.maxAppBundleBytes, 262144000)
    XCTAssertEqual(parsed.maxZipArtifactBytes, 157286400)
    XCTAssertEqual(parsed.sandboxMode, "workspaceReadWrite")
    XCTAssertEqual(parsed.sandboxBackend, "runtime-detected")
    XCTAssertEqual(parsed.sandboxFallback, "processOnlyWhenNativeUnavailable")
    XCTAssertEqual(parsed.sandboxNetworkDefault, "disabled")
    XCTAssertEqual(parsed.dailyDriverStageSource, "runtime/readiness")
    XCTAssertEqual(parsed.dailyDriverNextActionSource, "runtime/readiness")
    XCTAssertEqual(parsed.dailyDriverPresentation, "app-header-inspector")
    XCTAssertEqual(parsed.firstAppOpenActionContract, "map-plan-or-short-cowork-prompt")
    XCTAssertEqual(parsed.sourceCommit, sourceCommit)
  }

  func testManifestParsingRejectsUnknownPackageSchema() {
    XCTAssertNil(
      DistributionPackageMetadata.fromManifestData(
        manifestData(signing: "ad-hoc", schemaVersion: 2)
      )
    )
  }

  func testManifestParsingBackfillsDistributionTrustForOlderPackages() throws {
    let parsed = try XCTUnwrap(
      DistributionPackageMetadata.fromManifestData(
        manifestData(signing: "developer-id", distributionTrust: nil)
      )
    )

    XCTAssertEqual(parsed.distributionTrust, "developer-id-signed-notarized")
  }

  private func metadata(
    signing: String,
    distributionTrust: String? = nil
  ) throws -> DistributionPackageMetadata {
    try XCTUnwrap(
      DistributionPackageMetadata.fromManifestData(
        manifestData(signing: signing, distributionTrust: distributionTrust)
      )
    )
  }

  private func manifestData(
    signing: String,
    schemaVersion: Int = 1,
    distributionTrust: String? = "ad-hoc-not-notarized"
  ) -> Data {
    let distributionTrustLine = distributionTrust.map {
      """
      "distributionTrust": "\($0)",
      """
    } ?? ""
    return """
    {
      "architecture": "x86_64",
      \(distributionTrustLine)
      "minimumSystemVersion": "12.0",
      "modelDelivery": "in-app-download",
      "modelWeightsBundled": false,
      "amentiaAccountRequired": false,
      "defaultLocalExecutionSafetyMode": "askBeforeChange",
      "localExecutionSafetyModes": [
        "explore",
        "askBeforeChange",
        "approvedWorkspaceExecution"
      ],
      "sizeBudget": {
        "maxAppBundleBytes": 262144000,
        "maxZipArtifactBytes": 157286400
      },
      "sandboxMode": "workspaceReadWrite",
      "sandboxBackend": "runtime-detected",
      "sandboxFallback": "processOnlyWhenNativeUnavailable",
      "sandboxNetworkDefault": "disabled",
      "dailyDriverStageSource": "runtime/readiness",
      "dailyDriverNextActionSource": "runtime/readiness",
      "dailyDriverPresentation": "app-header-inspector",
      "firstAppOpenActionContract": "map-plan-or-short-cowork-prompt",
      "schemaVersion": \(schemaVersion),
      "sourceCommit": "\(sourceCommit)",
      "signing": "\(signing)"
    }
    """.data(using: .utf8)!
  }

  private let sourceCommit = "0123456789abcdef0123456789abcdef01234567"
}
