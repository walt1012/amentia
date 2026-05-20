import Foundation

struct RuntimeBridgeActiveLocalModelSelection {
  let manifestPath: String
  let modelPath: String
}

enum RuntimeBridgeLocalEnvironment {
  private static let activeModelManifestPathKey = "pith.activeModelManifestPath"
  private static let activeModelPathKey = "pith.activeModelPath"
  private static let activeModelInvalidationDetailKey = "pith.activeModelInvalidationDetail"
  private static let strippedRuntimeEnvironmentKeys = [
    "PITH_ENABLE_WEB_SEARCH_FIXTURE",
    "PITH_WEB_SEARCH_FIXTURE_PATH",
  ]

  static func localPluginInstallRootPath() -> String {
    AppSupportDirectories.pluginInstallDirectory().path
  }

  static func localModelStorageRootPath() -> String {
    AppSupportDirectories.localModelStorageDirectory().path
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
    stripRuntimeOnlyTestEnvironment(from: &environment)
    environment["PITH_DATA_DIR"] = AppSupportDirectories.storageDirectory().path
    environment["PITH_LOCAL_PLUGIN_DIR"] = AppSupportDirectories.pluginInstallDirectory().path
    applyBundleResourceEnvironment(to: &environment)
    if let activeModel = activeLocalModelSelection() {
      environment["PITH_MODEL_PACK_MANIFEST"] = activeModel.manifestPath
      environment["PITH_MODEL_PATH"] = activeModel.modelPath
      environment["PITH_LFM_MODEL_PATH"] = activeModel.modelPath
    }
    return environment
  }

  private static func stripRuntimeOnlyTestEnvironment(from environment: inout [String: String]) {
    for key in strippedRuntimeEnvironmentKeys {
      environment.removeValue(forKey: key)
    }
  }

  private static func applyBundleResourceEnvironment(to environment: inout [String: String]) {
    guard let resourceURL = Bundle.main.resourceURL else {
      return
    }

    let manager = FileManager.default
    let modelsURL = resourceURL.appendingPathComponent("models", isDirectory: true)
    if manager.fileExists(atPath: modelsURL.path) {
      environment["PITH_MODEL_PACK_ROOT"] = resourceURL.path
    }

    let pluginsURL = resourceURL.appendingPathComponent("plugins", isDirectory: true)
    if manager.fileExists(atPath: pluginsURL.path) {
      environment["PITH_PLUGIN_DIR"] = pluginsURL.path
    }

    let bundledLlamaURL = resourceURL
      .appendingPathComponent("tools", isDirectory: true)
      .appendingPathComponent("llama.cpp", isDirectory: true)
      .appendingPathComponent("llama-cli", isDirectory: false)
    if manager.fileExists(atPath: bundledLlamaURL.path) {
      environment["PITH_LLAMACPP_PATH"] = bundledLlamaURL.path
    }
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
