import Foundation

enum AppSupportDirectories {
  private static let supportRootOverrideKey = "PITH_APP_SUPPORT_DIR"

  static func rootDirectory() -> URL {
    if let overridePath = ProcessInfo.processInfo.environment[supportRootOverrideKey],
       !overridePath.isEmpty
    {
      return URL(fileURLWithPath: overridePath, isDirectory: true)
    }

    return defaultApplicationSupportBase()
      .appendingPathComponent("Pith", isDirectory: true)
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
      for directory in appOwnedDirectories() {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
      }
      return nil
    } catch {
      return "App support directory setup failed: \(error.localizedDescription)"
    }
  }

  private static func appOwnedDirectories() -> [URL] {
    [
      rootDirectory(),
      storageDirectory(),
      localModelStorageDirectory(),
      pluginInstallDirectory(),
      modelDownloadDirectory(),
    ]
  }

  private static func defaultApplicationSupportBase() -> URL {
    FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
      ?? URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)
  }
}
