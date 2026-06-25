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
      "No downloaded model files yet. Sessions, plugins, saved connections, preferences, and caches stay on this Mac."
    )
    XCTAssertTrue(summary.ownershipDetail.contains("plugins"))
    XCTAssertTrue(summary.ownershipDetail.contains("saved connection sign-ins"))
    XCTAssertTrue(summary.ownershipDetail.contains("caches"))
    XCTAssertTrue(summary.ownershipDetail.contains("window layout"))
    XCTAssertTrue(summary.ownershipDetail.contains("Project folders are never deleted"))
    XCTAssertTrue(summary.uninstallDetail.contains("Removing Amentia.app does not remove this data"))
    XCTAssertTrue(summary.uninstallDetail.contains("Delete All Local Data"))
    XCTAssertNil(summary.blockedDetail)
    XCTAssertEqual(summary.revealButtonTitle, "Show Local Data")
    XCTAssertEqual(summary.deleteButtonTitle, "Delete All Local Data...")
    XCTAssertTrue(summary.confirmationTitle.contains("Delete All Local Amentia Data"))
  }

  func testSummaryExplainsDownloadedModelStorage() {
    let summary = LocalDataSettingsPresenter.summary(
      LocalDataSettingsSnapshot(
        downloadedModelBytes: 222_662_560,
        canDeleteLocalData: true,
        localDataPath: "/Users/example/Library/Application Support/Amentia"
      )
    )

    XCTAssertTrue(summary.storageSummary.contains("Downloaded models use"))
    XCTAssertTrue(summary.storageSummary.contains("on this Mac"))
    XCTAssertTrue(
      summary.confirmationMessage.contains("project folders and repositories will not be deleted")
    )
    XCTAssertTrue(summary.confirmationMessage.contains("Amentia data"))
    XCTAssertTrue(summary.confirmationMessage.contains("from this Mac"))
    XCTAssertTrue(summary.confirmationMessage.contains("plugins"))
    XCTAssertTrue(summary.confirmationMessage.contains("saved connection sign-ins"))
    XCTAssertTrue(summary.confirmationMessage.contains("paused downloads"))
    XCTAssertTrue(summary.confirmationMessage.contains("caches"))
    XCTAssertTrue(summary.confirmationMessage.contains("saved window layout"))
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
    XCTAssertFalse(summary.blockedDetail?.contains("model downloads") == true)
    XCTAssertTrue(summary.blockedDetail?.contains("model checks") == true)
    XCTAssertTrue(summary.blockedDetail?.contains("deleting local data") == true)
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
      "Deleted Amentia local data. Restart Amentia to set up again."
    )
    XCTAssertFalse(reset.runtimeDetail.contains("/Users/example"))
    XCTAssertEqual(reset.timelineTitle, "Local Data Deleted")
    XCTAssertTrue(reset.timelineBody.contains("Amentia data"))
    XCTAssertTrue(reset.timelineBody.contains("Project folders on disk were not deleted"))
    XCTAssertTrue(reset.timelineBody.contains("plugins"))
    XCTAssertTrue(reset.timelineBody.contains("saved connection sign-ins"))
    XCTAssertTrue(reset.timelineBody.contains("paused downloads"))
    XCTAssertTrue(reset.timelineBody.contains("caches"))
    XCTAssertTrue(reset.timelineBody.contains("saved window layout"))
    XCTAssertTrue(reset.timelineBody.contains("Amentia support folders"))
    XCTAssertEqual(
      reset.attributes["appSupportPath"],
      "/Users/example/Library/Application Support/Amentia"
    )
    XCTAssertEqual(reset.attributes["remainingAppOwnedDirectoryCount"], "0")
  }

  func testResetCredentialCleanupErrorUsesUserFacingCopy() {
    let detail = AppDataResetError.credentialCleanupFailed(status: -34018)
      .localizedDescription

    XCTAssertTrue(detail.contains("saved connection sign-ins"))
    XCTAssertFalse(detail.contains("credentials"))
    XCTAssertFalse(detail.contains("Keychain"))
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
