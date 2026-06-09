@testable import PithApp
import XCTest

final class LocalDataSettingsPresenterTests: XCTestCase {
  func testSummaryExplainsEmptyLocalStorage() {
    let summary = LocalDataSettingsPresenter.summary(
      LocalDataSettingsSnapshot(
        downloadedModelBytes: 0,
        canDeleteLocalData: true,
        localDataPath: "/Users/example/Library/Application Support/Pith"
      )
    )

    XCTAssertEqual(
      summary.storageSummary,
      "No downloaded model files yet. Sessions, plugins, and preferences stay local."
    )
    XCTAssertTrue(summary.ownershipDetail.contains("Workspaces are never deleted"))
    XCTAssertNil(summary.blockedDetail)
    XCTAssertEqual(summary.revealButtonTitle, "Show Local Data")
    XCTAssertEqual(summary.deleteButtonTitle, "Delete Local Data...")
  }

  func testSummaryExplainsDownloadedModelStorage() {
    let summary = LocalDataSettingsPresenter.summary(
      LocalDataSettingsSnapshot(
        downloadedModelBytes: 229_312_224,
        canDeleteLocalData: true,
        localDataPath: "/Users/example/Library/Application Support/Pith"
      )
    )

    XCTAssertTrue(summary.storageSummary.contains("Downloaded models use"))
    XCTAssertTrue(summary.storageSummary.contains("on this Mac"))
    XCTAssertTrue(
      summary.confirmationMessage.contains("workspaces and repositories will not be deleted")
    )
  }

  func testSummaryExplainsBlockedDeletion() {
    let summary = LocalDataSettingsPresenter.summary(
      LocalDataSettingsSnapshot(
        downloadedModelBytes: 0,
        canDeleteLocalData: false,
        localDataPath: "/Users/example/Library/Application Support/Pith"
      )
    )

    XCTAssertFalse(summary.canDeleteLocalData)
    XCTAssertTrue(summary.blockedDetail?.contains("Finish active local work") == true)
  }
}
