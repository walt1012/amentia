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
      "No downloaded model files yet. Sessions, plugins, connections, and preferences stay local."
    )
    XCTAssertTrue(summary.ownershipDetail.contains("plugins"))
    XCTAssertTrue(summary.ownershipDetail.contains("connection credentials"))
    XCTAssertTrue(summary.ownershipDetail.contains("Project folders are never deleted"))
    XCTAssertTrue(summary.uninstallDetail.contains("Removing Pith.app does not remove this data"))
    XCTAssertTrue(summary.uninstallDetail.contains("Delete Local Data"))
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
      summary.confirmationMessage.contains("project folders and repositories will not be deleted")
    )
    XCTAssertTrue(summary.confirmationMessage.contains("from this Mac"))
    XCTAssertTrue(summary.confirmationMessage.contains("plugins"))
    XCTAssertTrue(summary.confirmationMessage.contains("connection credentials"))
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
    XCTAssertTrue(summary.blockedDetail?.contains("Finish active work") == true)
    XCTAssertTrue(summary.blockedDetail?.contains("plugin and connection operations") == true)
  }

  func testResetSummaryKeepsPathOutOfRuntimeDetail() {
    let reset = LocalDataSettingsPresenter.resetSummary(
      AppDataResetResult(
        appSupportPath: "/Users/example/Library/Application Support/Pith",
        recreatedDirectoryCount: 7
      )
    )

    XCTAssertEqual(
      reset.runtimeDetail,
      "Deleted Pith local data. Restart Pith to set up again."
    )
    XCTAssertFalse(reset.runtimeDetail.contains("/Users/example"))
    XCTAssertTrue(reset.timelineBody.contains("Project folders on disk were not deleted"))
    XCTAssertTrue(reset.timelineBody.contains("plugins"))
    XCTAssertTrue(reset.timelineBody.contains("connection credentials"))
    XCTAssertEqual(
      reset.attributes["appSupportPath"],
      "/Users/example/Library/Application Support/Pith"
    )
    XCTAssertEqual(reset.attributes["recreatedDirectoryCount"], "7")
  }
}
