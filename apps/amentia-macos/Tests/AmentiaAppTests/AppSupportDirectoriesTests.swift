import Foundation
@testable import AmentiaApp
import XCTest

final class AppSupportDirectoriesTests: XCTestCase {
  func testPrepareAppOwnedDirectoriesCreatesFirstUseLayout() throws {
    let rootURL = try makeTemporaryDirectory(prefix: "amentia-app-support")
    try FileManager.default.removeItem(at: rootURL)
    defer {
      try? FileManager.default.removeItem(at: rootURL)
    }

    try AppSupportDirectories.prepareAppOwnedDirectories(rootDirectory: rootURL)

    for directory in expectedDirectories(rootURL: rootURL) {
      var isDirectory = ObjCBool(false)
      XCTAssertTrue(FileManager.default.fileExists(atPath: directory.path, isDirectory: &isDirectory))
      XCTAssertTrue(isDirectory.boolValue)
    }
  }

  func testPrepareAppOwnedDirectoriesRejectsSymbolicLinkRoot() throws {
    let realRootURL = try makeTemporaryDirectory(prefix: "amentia-app-support")
    let symlinkURL = FileManager.default.temporaryDirectory
      .appendingPathComponent("amentia-app-support-symlink-\(UUID().uuidString)", isDirectory: true)
    defer {
      try? FileManager.default.removeItem(at: symlinkURL)
      try? FileManager.default.removeItem(at: realRootURL)
    }

    try FileManager.default.createSymbolicLink(at: symlinkURL, withDestinationURL: realRootURL)

    XCTAssertThrowsError(
      try AppSupportDirectories.prepareAppOwnedDirectories(rootDirectory: symlinkURL)
    ) { error in
      XCTAssertTrue(error.localizedDescription.contains("symbolic link"))
    }
  }

  func testDeleteLocalDataRemovesOnlyAppOwnedRootAndLeavesNoAppFolders() throws {
    let parentURL = try makeTemporaryDirectory(prefix: "amentia-app-support")
    let rootURL = parentURL.appendingPathComponent("Amentia", isDirectory: true)
    let outsideURL = parentURL.appendingPathComponent("workspace", isDirectory: true)
    defer {
      try? FileManager.default.removeItem(at: parentURL)
    }

    try AppSupportDirectories.prepareAppOwnedDirectories(rootDirectory: rootURL)
    try FileManager.default.createDirectory(at: outsideURL, withIntermediateDirectories: true)
    let modelFile = rootURL
      .appendingPathComponent("storage", isDirectory: true)
      .appendingPathComponent("models", isDirectory: true)
      .appendingPathComponent("local.gguf")
    try Data("model".utf8).write(to: modelFile)
    let workspaceFile = outsideURL.appendingPathComponent("README.md")
    try Data("workspace".utf8).write(to: workspaceFile)

    let result = try AppDataResetService.deleteLocalData(
      rootDirectory: rootURL,
      allowsNonDefaultRoot: true
    )

    XCTAssertEqual(result.appSupportPath, rootURL.path)
    XCTAssertEqual(result.remainingAppOwnedDirectoryCount, 0)
    XCTAssertFalse(FileManager.default.fileExists(atPath: modelFile.path))
    XCTAssertFalse(FileManager.default.fileExists(atPath: rootURL.path))
    XCTAssertTrue(FileManager.default.fileExists(atPath: workspaceFile.path))
  }

  private func expectedDirectories(rootURL: URL) -> [URL] {
    [
      rootURL,
      rootURL.appendingPathComponent("storage", isDirectory: true),
      rootURL
        .appendingPathComponent("storage", isDirectory: true)
        .appendingPathComponent("models", isDirectory: true),
      rootURL.appendingPathComponent("plugins", isDirectory: true),
      rootURL.appendingPathComponent("model-downloads", isDirectory: true),
    ]
  }

}
