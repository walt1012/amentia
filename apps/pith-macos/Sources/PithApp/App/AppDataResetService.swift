import Foundation
import Security

struct AppDataResetResult: Equatable {
  let appSupportPath: String
  let remainingAppOwnedDirectoryCount: Int
}

enum AppDataResetService {
  private static let connectorCredentialService = "app.pith.plugin-connectors"
  private static let appBundleIdentifier = "app.amentia.Amentia"
  private static let appDisplayName = "Amentia"
  private static let legacyAppBundleIdentifier = "app.pith.Pith"
  private static let legacyAppDisplayName = "Pith"
  private static let legacyAppSupportDirectoryName = "Pith"

  static func deleteLocalData(
    rootDirectory: URL = AppSupportDirectories.rootDirectory(),
    allowsNonDefaultRoot: Bool = false
  ) throws -> AppDataResetResult {
    try validateResetRoot(rootDirectory, allowsNonDefaultRoot: allowsNonDefaultRoot)
    try removeConnectorCredentials()
    try removeAppSupportRoot(rootDirectory)
    clearKnownPreferences()
    if shouldRemoveSystemResidue(allowsNonDefaultRoot: allowsNonDefaultRoot) {
      try removeLegacyAppSupportRootIfPresent()
      try removeSystemResidue()
    }
    return AppDataResetResult(
      appSupportPath: rootDirectory.path,
      remainingAppOwnedDirectoryCount: 0
    )
  }

  static func clearKnownPreferences() {
    AppPreferences.clearStoredPreferences()
    RuntimeBridgeLocalEnvironment.clearActiveLocalModel()
    LocalModelCatalog.clearPausedDownload()
    LocalModelVerificationStampStore.forgetAllVerifiedModels()
  }

  private static func removeAppSupportRoot(_ rootDirectory: URL) throws {
    let manager = FileManager.default
    guard manager.fileExists(atPath: rootDirectory.path) else {
      return
    }

    try manager.removeItem(at: rootDirectory)
  }

  private static func removeLegacyAppSupportRootIfPresent() throws {
    guard let applicationSupportDirectory = FileManager.default.urls(
      for: .applicationSupportDirectory,
      in: .userDomainMask
    ).first else {
      return
    }
    let legacyRoot = applicationSupportDirectory
      .appendingPathComponent(legacyAppSupportDirectoryName, isDirectory: true)
      .standardizedFileURL
    guard legacyRoot.lastPathComponent == legacyAppSupportDirectoryName else {
      return
    }
    if (try? FileManager.default.destinationOfSymbolicLink(atPath: legacyRoot.path)) != nil {
      throw AppDataResetError.symbolicLink(path: legacyRoot.path)
    }
    try removeIfPresent(legacyRoot)
  }

  private static func shouldRemoveSystemResidue(allowsNonDefaultRoot: Bool) -> Bool {
    let hasOverride = ProcessInfo.processInfo.environment["PITH_APP_SUPPORT_DIR"]?.isEmpty == false
    return !allowsNonDefaultRoot && !hasOverride
  }

  private static func removeSystemResidue() throws {
    UserDefaults.standard.removePersistentDomain(forName: appBundleIdentifier)

    guard let libraryDirectory = userLibraryDirectory() else {
      return
    }

    for path in systemResiduePaths(libraryDirectory: libraryDirectory) {
      try removeIfPresent(path)
    }
  }

  static func systemResiduePaths(libraryDirectory: URL) -> [URL] {
    systemResiduePaths(
      libraryDirectory: libraryDirectory,
      bundleIdentifier: appBundleIdentifier,
      displayName: appDisplayName
    ) + systemResiduePaths(
      libraryDirectory: libraryDirectory,
      bundleIdentifier: legacyAppBundleIdentifier,
      displayName: legacyAppDisplayName
    )
  }

  private static func systemResiduePaths(
    libraryDirectory: URL,
    bundleIdentifier: String,
    displayName: String
  ) -> [URL] {
    [
      libraryDirectory
        .appendingPathComponent("Preferences", isDirectory: true)
        .appendingPathComponent("\(bundleIdentifier).plist", isDirectory: false),
      libraryDirectory
        .appendingPathComponent("Caches", isDirectory: true)
        .appendingPathComponent(bundleIdentifier, isDirectory: true),
      libraryDirectory
        .appendingPathComponent("Caches", isDirectory: true)
        .appendingPathComponent(displayName, isDirectory: true),
      libraryDirectory
        .appendingPathComponent("Saved Application State", isDirectory: true)
        .appendingPathComponent("\(bundleIdentifier).savedState", isDirectory: true),
    ]
  }

  private static func removeIfPresent(_ url: URL) throws {
    let manager = FileManager.default
    guard manager.fileExists(atPath: url.path) else {
      return
    }

    try manager.removeItem(at: url)
  }

  private static func userLibraryDirectory() -> URL? {
    FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask).first
  }

  private static func removeConnectorCredentials() throws {
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: connectorCredentialService,
    ]
    let status = SecItemDelete(query as CFDictionary)
    guard status == errSecSuccess || status == errSecItemNotFound else {
      throw AppDataResetError.credentialCleanupFailed(status: status)
    }
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
    guard allowsNonDefaultRoot || hasOverride || standardized.lastPathComponent == "Amentia" else {
      throw AppDataResetError.unsafeRoot(path: standardized.path)
    }
  }
}

enum AppDataResetError: LocalizedError, Equatable {
  case unsafeRoot(path: String)
  case symbolicLink(path: String)
  case credentialCleanupFailed(status: OSStatus)

  var errorDescription: String? {
    switch self {
    case .unsafeRoot(let path):
      return "Refusing to delete an unsafe app data path: \(path)"
    case .symbolicLink(let path):
      return "Refusing to delete app data through a symbolic link: \(path)"
    case .credentialCleanupFailed(let status):
      return "Failed to remove saved connection credentials from Keychain: \(status)"
    }
  }
}
