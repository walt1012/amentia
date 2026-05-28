@testable import PithApp
import XCTest

final class DistributionTrustPresenterTests: XCTestCase {
  func testAdHocBuildExplainsGatekeeperPath() throws {
    let summary = DistributionTrustPresenter.summary(
      try metadata(signing: "ad-hoc")
    )

    XCTAssertEqual(summary.title, "Untrusted Ad-Hoc Build")
    XCTAssertTrue(summary.detail.contains("Open Anyway"))
    XCTAssertTrue(summary.detail.contains("model weights are not bundled"))
    XCTAssertTrue(summary.detail.contains("no Pith account required"))
    XCTAssertTrue(summary.detail.contains("local execution mode: ask-before-change"))
    XCTAssertTrue(summary.detail.contains("package size budget"))
    XCTAssertTrue(summary.detail.contains("app <= 250 MiB"))
    XCTAssertTrue(summary.detail.contains("installer artifact <= 150 MiB"))
    XCTAssertTrue(summary.detail.contains("process-only fallback"))
    XCTAssertTrue(summary.detail.contains("daily-driver next action"))
    XCTAssertTrue(summary.detail.contains("source: 0123456789ab"))
    XCTAssertTrue(summary.setupDetail?.contains("Control-click Pith.app") == true)
  }

  func testDeveloperIDBuildAvoidsManualGatekeeperCopy() throws {
    let summary = DistributionTrustPresenter.summary(
      try metadata(signing: "developer-id")
    )

    XCTAssertEqual(summary.title, "Trusted Installer")
    XCTAssertTrue(summary.summary.contains("Developer ID signed and notarized"))
    XCTAssertNil(summary.setupDetail)
    XCTAssertFalse(summary.detail.contains("Open Anyway"))
  }

  func testDevelopmentFallbackNamesReleaseAssets() {
    let summary = DistributionTrustPresenter.summary(.development)

    XCTAssertEqual(summary.title, "Development Build")
    XCTAssertTrue(summary.detail.contains("README-FIRST.txt"))
    XCTAssertTrue(summary.detail.contains("release manifest"))
  }

  func testManifestParsingKeepsPackageContract() throws {
    let parsed = try XCTUnwrap(
      DistributionPackageMetadata.fromManifestData(manifestData(signing: "ad-hoc"))
    )

    XCTAssertEqual(parsed.signing, "ad-hoc")
    XCTAssertEqual(parsed.schemaVersion, 1)
    XCTAssertEqual(parsed.architecture, "x86_64")
    XCTAssertEqual(parsed.minimumSystemVersion, "12.0")
    XCTAssertEqual(parsed.modelDelivery, "in-app-download")
    XCTAssertFalse(parsed.modelWeightsBundled)
    XCTAssertFalse(parsed.pithAccountRequired)
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
    XCTAssertEqual(parsed.sourceCommit, sourceCommit)
  }

  func testManifestParsingRejectsUnknownPackageSchema() {
    XCTAssertNil(
      DistributionPackageMetadata.fromManifestData(
        manifestData(signing: "ad-hoc", schemaVersion: 2)
      )
    )
  }

  private func metadata(signing: String) throws -> DistributionPackageMetadata {
    try XCTUnwrap(
      DistributionPackageMetadata.fromManifestData(manifestData(signing: signing))
    )
  }

  private func manifestData(signing: String, schemaVersion: Int = 1) -> Data {
    """
    {
      "architecture": "x86_64",
      "minimumSystemVersion": "12.0",
      "modelDelivery": "in-app-download",
      "modelWeightsBundled": false,
      "pithAccountRequired": false,
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
      "schemaVersion": \(schemaVersion),
      "sourceCommit": "\(sourceCommit)",
      "signing": "\(signing)"
    }
    """.data(using: .utf8)!
  }

  private let sourceCommit = "0123456789abcdef0123456789abcdef01234567"
}
