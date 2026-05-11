import Foundation

struct RuntimeBridgeActiveLocalModelSelection {
  let manifestPath: String
  let modelPath: String
}

enum RuntimeBridgeLocalEnvironment {
  private static let activeModelManifestPathKey = "pith.activeModelManifestPath"
  private static let activeModelPathKey = "pith.activeModelPath"
  private static let activeModelInvalidationDetailKey = "pith.activeModelInvalidationDetail"

  static func localPluginInstallRootPath() -> String {
    pluginDirectory().path
  }

  static func localModelStorageRootPath() -> String {
    storageDirectory()
      .appendingPathComponent("models", isDirectory: true)
      .path
  }

  static func activeLocalModelPath() -> String? {
    activeLocalModelSelection()?.modelPath
  }

  static func configureActiveLocalModel(manifestPath: String, modelPath: String) {
    UserDefaults.standard.set(manifestPath, forKey: activeModelManifestPathKey)
    UserDefaults.standard.set(modelPath, forKey: activeModelPathKey)
    UserDefaults.standard.removeObject(forKey: activeModelInvalidationDetailKey)
  }

  static func clearActiveLocalModel() {
    clearActiveLocalModel(invalidationDetail: nil)
  }

  static func consumeActiveLocalModelInvalidationDetail() -> String? {
    let defaults = UserDefaults.standard
    let detail = defaults.string(forKey: activeModelInvalidationDetailKey)
    defaults.removeObject(forKey: activeModelInvalidationDetailKey)
    return detail
  }

  private static func clearActiveLocalModel(invalidationDetail: String?) {
    UserDefaults.standard.removeObject(forKey: activeModelManifestPathKey)
    UserDefaults.standard.removeObject(forKey: activeModelPathKey)
    if let invalidationDetail {
      UserDefaults.standard.set(invalidationDetail, forKey: activeModelInvalidationDetailKey)
    } else {
      UserDefaults.standard.removeObject(forKey: activeModelInvalidationDetailKey)
    }
  }

  static func runtimeEnvironment() -> [String: String] {
    var environment = ProcessInfo.processInfo.environment
    environment["PITH_DATA_DIR"] = storageDirectory().path
    environment["PITH_LOCAL_PLUGIN_DIR"] = pluginDirectory().path
    if let activeModel = activeLocalModelSelection() {
      environment["PITH_MODEL_PACK_MANIFEST"] = activeModel.manifestPath
      environment["PITH_MODEL_PATH"] = activeModel.modelPath
      environment["PITH_LFM_MODEL_PATH"] = activeModel.modelPath
    }
    return environment
  }

  private static func activeLocalModelSelection() -> RuntimeBridgeActiveLocalModelSelection? {
    let defaults = UserDefaults.standard
    guard let manifestPath = defaults.string(forKey: activeModelManifestPathKey),
          !manifestPath.isEmpty,
          let modelPath = defaults.string(forKey: activeModelPathKey),
          !modelPath.isEmpty
    else {
      return nil
    }

    let manager = FileManager.default
    guard manager.fileExists(atPath: manifestPath),
          manager.fileExists(atPath: modelPath)
    else {
      clearActiveLocalModel(
        invalidationDetail:
          "The saved active local model was reset because its manifest or GGUF file no longer exists. Choose or download a model to continue."
      )
      return nil
    }

    guard LocalModelCatalog.isVerifiedInstalledSelection(
      storageRootPath: localModelStorageRootPath(),
      modelPath: modelPath,
      manifestPath: manifestPath
    ) else {
      clearActiveLocalModel(
        invalidationDetail:
          "The saved active local model was reset because its manifest and GGUF file no longer match the verified catalog. Choose or download a model to continue."
      )
      return nil
    }

    return RuntimeBridgeActiveLocalModelSelection(manifestPath: manifestPath, modelPath: modelPath)
  }

  private static func storageDirectory() -> URL {
    let baseDirectory =
      FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
      ?? URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)

    return baseDirectory
      .appendingPathComponent("Pith", isDirectory: true)
      .appendingPathComponent("storage", isDirectory: true)
  }

  private static func pluginDirectory() -> URL {
    let baseDirectory =
      FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
      ?? URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)

    return baseDirectory
      .appendingPathComponent("Pith", isDirectory: true)
      .appendingPathComponent("plugins", isDirectory: true)
  }
}

extension RuntimeBridge {
  func localPluginInstallRootPath() -> String {
    RuntimeBridgeLocalEnvironment.localPluginInstallRootPath()
  }

  func localModelStorageRootPath() -> String {
    RuntimeBridgeLocalEnvironment.localModelStorageRootPath()
  }

  func activeLocalModelPath() -> String? {
    RuntimeBridgeLocalEnvironment.activeLocalModelPath()
  }

  func configureActiveLocalModel(manifestPath: String, modelPath: String) {
    RuntimeBridgeLocalEnvironment.configureActiveLocalModel(
      manifestPath: manifestPath,
      modelPath: modelPath
    )
  }

  func clearActiveLocalModel() {
    RuntimeBridgeLocalEnvironment.clearActiveLocalModel()
  }

  func consumeActiveLocalModelInvalidationDetail() -> String? {
    RuntimeBridgeLocalEnvironment.consumeActiveLocalModelInvalidationDetail()
  }
}
