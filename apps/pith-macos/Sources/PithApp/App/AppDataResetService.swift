import Foundation

struct AppDataResetResult: Equatable {
  let appSupportPath: String
  let recreatedDirectoryCount: Int
}

enum AppDataResetService {
  static func deleteLocalData(
    rootDirectory: URL = AppSupportDirectories.rootDirectory(),
    allowsNonDefaultRoot: Bool = false
  ) throws -> AppDataResetResult {
    try validateResetRoot(rootDirectory, allowsNonDefaultRoot: allowsNonDefaultRoot)
    try removeAppSupportRoot(rootDirectory)
    clearKnownPreferences()
    try AppSupportDirectories.prepareAppOwnedDirectories(rootDirectory: rootDirectory)
    return AppDataResetResult(
      appSupportPath: rootDirectory.path,
      recreatedDirectoryCount: AppSupportDirectories.appOwnedDirectoryCount
    )
  }

  static func clearKnownPreferences() {
    AppPreferences.clearStoredPreferences()
    RuntimeBridgeLocalEnvironment.clearActiveLocalModel()
    LocalModelCatalog.clearPausedDownload()
    LocalModelVerificationStampStore.forgetVerifiedModels(
      modelIDs: LocalModelCatalog.items().map(\.id)
    )
  }

  private static func removeAppSupportRoot(_ rootDirectory: URL) throws {
    let manager = FileManager.default
    guard manager.fileExists(atPath: rootDirectory.path) else {
      return
    }

    try manager.removeItem(at: rootDirectory)
  }

  private static func validateResetRoot(
    _ rootDirectory: URL,
    allowsNonDefaultRoot: Bool
  ) throws {
    let standardized = rootDirectory.standardizedFileURL
    guard standardized.pathComponents.count >= 4 else {
      throw AppDataResetError.unsafeRoot(path: standardized.path)
    }
    if (try? FileManager.default.destinationOfSymbolicLink(atPath: standardized.path)) != nil {
      throw AppDataResetError.symbolicLink(path: standardized.path)
    }

    let hasOverride = ProcessInfo.processInfo.environment["PITH_APP_SUPPORT_DIR"]?.isEmpty == false
    guard allowsNonDefaultRoot || hasOverride || standardized.lastPathComponent == "Pith" else {
      throw AppDataResetError.unsafeRoot(path: standardized.path)
    }
  }
}

enum AppDataResetError: LocalizedError, Equatable {
  case unsafeRoot(path: String)
  case symbolicLink(path: String)

  var errorDescription: String? {
    switch self {
    case .unsafeRoot(let path):
      return "Refusing to delete an unsafe app data path: \(path)"
    case .symbolicLink(let path):
      return "Refusing to delete app data through a symbolic link: \(path)"
    }
  }
}
