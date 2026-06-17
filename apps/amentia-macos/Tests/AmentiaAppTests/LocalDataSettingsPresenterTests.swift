@testable import AmentiaApp
import XCTest

final class LocalDataSettingsPresenterTests: XCTestCase {
  func testClearStoredPreferencesRemovesInternalAppKeys() {
    let defaults = UserDefaults.standard
    defaults.set("workspace", forKey: "amentia.lastWorkspacePath")
    defaults.set(true, forKey: "amentia.inspector.localModelExpanded")
    defaults.set("stamp", forKey: "amentia.verifiedLocalModel.example")
    defaults.set("keep", forKey: "other.preference")

    AppPreferences.clearStoredPreferences()

    XCTAssertNil(defaults.object(forKey: "amentia.lastWorkspacePath"))
    XCTAssertNil(defaults.object(forKey: "amentia.inspector.localModelExpanded"))
    XCTAssertNil(defaults.object(forKey: "amentia.verifiedLocalModel.example"))
    XCTAssertEqual(defaults.string(forKey: "other.preference"), "keep")

    defaults.removeObject(forKey: "other.preference")
  }

  func testSummaryExplainsEmptyLocalStorage() {
    let summary = LocalDataSettingsPresenter.summary(
      LocalDataSettingsSnapshot(
        downloadedModelBytes: 0,
        canDeleteLocalData: true,
        localDataPath: "/Users/example/Library/Application Support/Amentia"
      )
    )

    XCTAssertEqual(
      summary.storageSummary,
      "No downloaded model files yet. Sessions, plugins, connections, preferences, and caches stay on this Mac."
    )
    XCTAssertTrue(summary.ownershipDetail.contains("plugins"))
    XCTAssertTrue(summary.ownershipDetail.contains("connection credentials"))
    XCTAssertTrue(summary.ownershipDetail.contains("caches"))
    XCTAssertTrue(summary.ownershipDetail.contains("window state"))
    XCTAssertTrue(summary.ownershipDetail.contains("Project folders are never deleted"))
    XCTAssertTrue(summary.uninstallDetail.contains("Removing Amentia.app does not remove this data"))
    XCTAssertTrue(summary.uninstallDetail.contains("Reset Amentia"))
    XCTAssertNil(summary.blockedDetail)
    XCTAssertEqual(summary.revealButtonTitle, "Show Amentia Data")
    XCTAssertEqual(summary.deleteButtonTitle, "Delete All Amentia Data...")
    XCTAssertTrue(summary.confirmationTitle.contains("Delete All Amentia Data"))
  }

  func testSummaryExplainsDownloadedModelStorage() {
    let summary = LocalDataSettingsPresenter.summary(
      LocalDataSettingsSnapshot(
        downloadedModelBytes: 229_312_224,
        canDeleteLocalData: true,
        localDataPath: "/Users/example/Library/Application Support/Amentia"
      )
    )

    XCTAssertTrue(summary.storageSummary.contains("Downloaded models use"))
    XCTAssertTrue(summary.storageSummary.contains("on this Mac"))
    XCTAssertTrue(
      summary.confirmationMessage.contains("project folders and repositories will not be deleted")
    )
    XCTAssertTrue(summary.confirmationMessage.contains("all app-owned local data"))
    XCTAssertTrue(summary.confirmationMessage.contains("from this Mac"))
    XCTAssertTrue(summary.confirmationMessage.contains("plugins"))
    XCTAssertTrue(summary.confirmationMessage.contains("connection credentials"))
    XCTAssertTrue(summary.confirmationMessage.contains("paused downloads"))
    XCTAssertTrue(summary.confirmationMessage.contains("caches"))
    XCTAssertTrue(summary.confirmationMessage.contains("saved app state"))
  }

  func testSummaryExplainsBlockedDeletion() {
    let summary = LocalDataSettingsPresenter.summary(
      LocalDataSettingsSnapshot(
        downloadedModelBytes: 0,
        canDeleteLocalData: false,
        localDataPath: "/Users/example/Library/Application Support/Amentia"
      )
    )

    XCTAssertFalse(summary.canDeleteLocalData)
    XCTAssertTrue(summary.blockedDetail?.contains("Finish active work") == true)
    XCTAssertTrue(summary.blockedDetail?.contains("resetting Amentia") == true)
    XCTAssertTrue(summary.blockedDetail?.contains("plugin and connection operations") == true)
  }

  func testResetSummaryKeepsPathOutOfRuntimeDetail() {
    let reset = LocalDataSettingsPresenter.resetSummary(
      AppDataResetResult(
        appSupportPath: "/Users/example/Library/Application Support/Amentia",
        remainingAppOwnedDirectoryCount: 0
      )
    )

    XCTAssertEqual(
      reset.runtimeDetail,
      "Reset Amentia. Restart Amentia to set up again."
    )
    XCTAssertFalse(reset.runtimeDetail.contains("/Users/example"))
    XCTAssertTrue(reset.timelineBody.contains("all app-owned local data"))
    XCTAssertTrue(reset.timelineBody.contains("Project folders on disk were not deleted"))
    XCTAssertTrue(reset.timelineBody.contains("plugins"))
    XCTAssertTrue(reset.timelineBody.contains("connection credentials"))
    XCTAssertTrue(reset.timelineBody.contains("paused downloads"))
    XCTAssertTrue(reset.timelineBody.contains("caches"))
    XCTAssertTrue(reset.timelineBody.contains("saved app state"))
    XCTAssertTrue(reset.timelineBody.contains("app-owned folders"))
    XCTAssertEqual(
      reset.attributes["appSupportPath"],
      "/Users/example/Library/Application Support/Amentia"
    )
    XCTAssertEqual(reset.attributes["remainingAppOwnedDirectoryCount"], "0")
  }

  func testSystemResiduePathsIncludeOnlyAmentiaBundle() {
    let libraryURL = URL(fileURLWithPath: "/Users/example/Library", isDirectory: true)
    let paths = AppDataResetService.systemResiduePaths(libraryDirectory: libraryURL)
      .map(\.path)

    XCTAssertEqual(
      paths,
      [
        "/Users/example/Library/Preferences/app.amentia.Amentia.plist",
        "/Users/example/Library/Caches/app.amentia.Amentia",
        "/Users/example/Library/Caches/Amentia",
        "/Users/example/Library/Saved Application State/app.amentia.Amentia.savedState",
      ]
    )
  }
}
