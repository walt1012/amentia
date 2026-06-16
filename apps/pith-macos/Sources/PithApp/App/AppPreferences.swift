import Foundation

enum AppPreferences {
  private static let lastWorkspacePathKey = "pith.lastWorkspacePath"
  private static let selectedSetupModelIDKey = "pith.selectedSetupModelID"
  private static let localExecutionSafetyModeKey = "pith.localExecutionSafetyMode"
  private static let appPreferenceKeyPrefix = "pith."

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

  static func storedLocalExecutionSafetyMode() -> String {
    LocalExecutionSafetyModePresenter.validMode(
      UserDefaults.standard.string(forKey: localExecutionSafetyModeKey)
    )
  }

  static func storeLocalExecutionSafetyMode(_ mode: String) {
    UserDefaults.standard.set(
      LocalExecutionSafetyModePresenter.validMode(mode),
      forKey: localExecutionSafetyModeKey
    )
  }

  static func clearStoredPreferences() {
    let defaults = UserDefaults.standard
    for key in defaults.dictionaryRepresentation().keys where key.hasPrefix(appPreferenceKeyPrefix) {
      defaults.removeObject(forKey: key)
    }
  }
}
