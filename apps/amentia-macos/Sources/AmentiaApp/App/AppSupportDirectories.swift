import Foundation

enum AppSupportDirectories {
  private static let supportRootOverrideKey = "AMENTIA_APP_SUPPORT_DIR"
  private static let currentSupportDirectoryName = "Amentia"
  static let appOwnedDirectoryCount = 5

  static func rootDirectory() -> URL {
    if let overridePath = ProcessInfo.processInfo.environment[supportRootOverrideKey],
       !overridePath.isEmpty
    {
      return URL(fileURLWithPath: overridePath, isDirectory: true)
    }

    return defaultApplicationSupportBase()
      .appendingPathComponent(currentSupportDirectoryName, isDirectory: true)
  }

  static func storageDirectory() -> URL {
    rootDirectory().appendingPathComponent("storage", isDirectory: true)
  }

  static func localModelStorageDirectory() -> URL {
    storageDirectory().appendingPathComponent("models", isDirectory: true)
  }

  static func pluginInstallDirectory() -> URL {
    rootDirectory().appendingPathComponent("plugins", isDirectory: true)
  }

  static func modelDownloadDirectory() -> URL {
    rootDirectory().appendingPathComponent("model-downloads", isDirectory: true)
  }

  static func prepareAppOwnedDirectories() -> String? {
    do {
      try prepareAppOwnedDirectories(rootDirectory: rootDirectory())
      return nil
    } catch {
      return "App support directory setup failed: \(error.localizedDescription)"
    }
  }

  static func prepareAppOwnedDirectories(rootDirectory: URL) throws {
    for directory in appOwnedDirectories(rootDirectory: rootDirectory) {
      try prepareAppOwnedDirectory(directory)
    }
  }

  private static func appOwnedDirectories(rootDirectory: URL) -> [URL] {
    [
      rootDirectory,
      rootDirectory.appendingPathComponent("storage", isDirectory: true),
      rootDirectory
        .appendingPathComponent("storage", isDirectory: true)
        .appendingPathComponent("models", isDirectory: true),
      rootDirectory.appendingPathComponent("plugins", isDirectory: true),
      rootDirectory.appendingPathComponent("model-downloads", isDirectory: true),
    ]
  }

  private static func prepareAppOwnedDirectory(_ directory: URL) throws {
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    try validateAppOwnedDirectory(directory)
  }

  private static func validateAppOwnedDirectory(_ directory: URL) throws {
    if (try? FileManager.default.destinationOfSymbolicLink(atPath: directory.path)) != nil {
      throw AppSupportDirectoryError.symbolicLink(path: directory.path)
    }

    var isDirectory = ObjCBool(false)
    guard FileManager.default.fileExists(atPath: directory.path, isDirectory: &isDirectory),
          isDirectory.boolValue
    else {
      throw AppSupportDirectoryError.notDirectory(path: directory.path)
    }
  }

  private static func defaultApplicationSupportBase() -> URL {
    FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
      ?? URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)
  }
}

private enum AppSupportDirectoryError: LocalizedError {
  case notDirectory(path: String)
  case symbolicLink(path: String)

  var errorDescription: String? {
    switch self {
    case .notDirectory(let path):
      return "App support path is not a directory: \(path)"
    case .symbolicLink(let path):
      return "App support path must be a real directory, not a symbolic link: \(path)"
    }
  }
}
