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
    XCTAssertEqual(parsed.architecture, "x86_64")
    XCTAssertEqual(parsed.minimumSystemVersion, "12.0")
    XCTAssertEqual(parsed.modelDelivery, "in-app-download")
    XCTAssertFalse(parsed.modelWeightsBundled)
  }

  private func metadata(signing: String) throws -> DistributionPackageMetadata {
    try XCTUnwrap(
      DistributionPackageMetadata.fromManifestData(manifestData(signing: signing))
    )
  }

  private func manifestData(signing: String) -> Data {
    """
    {
      "architecture": "x86_64",
      "minimumSystemVersion": "12.0",
      "modelDelivery": "in-app-download",
      "modelWeightsBundled": false,
      "signing": "\(signing)"
    }
    """.data(using: .utf8)!
  }
}
