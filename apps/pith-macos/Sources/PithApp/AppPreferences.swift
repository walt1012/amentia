import Foundation

enum AppPreferences {
  private static let lastWorkspacePathKey = "pith.lastWorkspacePath"
  private static let selectedSetupModelIDKey = "pith.selectedSetupModelID"

  static func storedSelectedSetupModelID(matching models: [LocalModelSummary]) -> String? {
    guard let modelID = UserDefaults.standard.string(forKey: selectedSetupModelIDKey),
          models.contains(where: { $0.id == modelID })
    else {
      return nil
    }

    return modelID
  }

  static func storeSelectedSetupModelID(_ modelID: String) {
    UserDefaults.standard.set(modelID, forKey: selectedSetupModelIDKey)
  }

  static func storedLastWorkspacePath() -> String? {
    guard let path = UserDefaults.standard.string(forKey: lastWorkspacePathKey),
          !path.isEmpty
    else {
      return nil
    }

    return path
  }

  static func storeLastWorkspacePath(_ path: String) {
    guard !path.isEmpty else {
      return
    }

    UserDefaults.standard.set(path, forKey: lastWorkspacePathKey)
  }

  static func clearLastWorkspacePath() {
    UserDefaults.standard.removeObject(forKey: lastWorkspacePathKey)
  }
}
