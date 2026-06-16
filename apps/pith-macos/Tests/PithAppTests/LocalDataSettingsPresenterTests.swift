@testable import PithApp
import XCTest

final class LocalDataSettingsPresenterTests: XCTestCase {
  func testClearStoredPreferencesRemovesAllPithKeys() {
    let defaults = UserDefaults.standard
    defaults.set("workspace", forKey: "pith.lastWorkspacePath")
    defaults.set(true, forKey: "pith.inspector.localModelExpanded")
    defaults.set("stamp", forKey: "pith.verifiedLocalModel.example")
    defaults.set("keep", forKey: "other.preference")

    AppPreferences.clearStoredPreferences()

    XCTAssertNil(defaults.object(forKey: "pith.lastWorkspacePath"))
    XCTAssertNil(defaults.object(forKey: "pith.inspector.localModelExpanded"))
    XCTAssertNil(defaults.object(forKey: "pith.verifiedLocalModel.example"))
    XCTAssertEqual(defaults.string(forKey: "other.preference"), "keep")

    defaults.removeObject(forKey: "other.preference")
  }

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
      "No downloaded model files yet. Sessions, plugins, connections, and preferences stay on this Mac."
    )
    XCTAssertTrue(summary.ownershipDetail.contains("plugins"))
    XCTAssertTrue(summary.ownershipDetail.contains("connection credentials"))
    XCTAssertTrue(summary.ownershipDetail.contains("Project folders are never deleted"))
    XCTAssertTrue(summary.uninstallDetail.contains("Removing Pith.app does not remove this data"))
    XCTAssertTrue(summary.uninstallDetail.contains("Reset Pith"))
    XCTAssertNil(summary.blockedDetail)
    XCTAssertEqual(summary.revealButtonTitle, "Show Pith Data")
    XCTAssertEqual(summary.deleteButtonTitle, "Reset Pith...")
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
    XCTAssertTrue(summary.confirmationMessage.contains("paused downloads"))
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
    XCTAssertTrue(summary.blockedDetail?.contains("resetting Pith") == true)
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
      "Reset Pith. Restart Pith to set up again."
    )
    XCTAssertFalse(reset.runtimeDetail.contains("/Users/example"))
    XCTAssertTrue(reset.timelineBody.contains("Project folders on disk were not deleted"))
    XCTAssertTrue(reset.timelineBody.contains("plugins"))
    XCTAssertTrue(reset.timelineBody.contains("connection credentials"))
    XCTAssertTrue(reset.timelineBody.contains("paused downloads"))
    XCTAssertEqual(
      reset.attributes["appSupportPath"],
      "/Users/example/Library/Application Support/Pith"
    )
    XCTAssertEqual(reset.attributes["recreatedDirectoryCount"], "7")
  }
}
