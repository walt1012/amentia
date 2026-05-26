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
      "schemaVersion": \(schemaVersion),
      "sourceCommit": "\(sourceCommit)",
      "signing": "\(signing)"
    }
    """.data(using: .utf8)!
  }

  private let sourceCommit = "0123456789abcdef0123456789abcdef01234567"
}
