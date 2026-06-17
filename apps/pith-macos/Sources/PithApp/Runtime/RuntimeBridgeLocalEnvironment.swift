import Foundation

struct RuntimeBridgeActiveLocalModelSelection {
  let manifestPath: String
  let modelPath: String
}

enum RuntimeBridgeLocalEnvironment {
  private static let activeModelManifestPathKey = "pith.activeModelManifestPath"
  private static let activeModelPathKey = "pith.activeModelPath"
  private static let activeModelInvalidationDetailKey = "pith.activeModelInvalidationDetail"
  private static let appEnvironmentPrefix = "PITH_"

  static func localPluginInstallRootPath() -> String {
    AppSupportDirectories.pluginInstallDirectory().path
  }

  static func localModelStorageRootPath() -> String {
    AppSupportDirectories.localModelStorageDirectory().path
  }

  static func activeLocalModelPath() -> String? {
    let defaults = UserDefaults.standard
    return defaults.string(forKey: activeModelPathKey)
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

  static func runtimeEnvironment() async -> [String: String] {
    var environment = sanitizedInheritedEnvironment()
    if let setupError = AppSupportDirectories.prepareAppOwnedDirectories() {
      environment["PITH_APP_SUPPORT_SETUP_ERROR"] = setupError
    }
    environment["PITH_DATA_DIR"] = AppSupportDirectories.storageDirectory().path
    environment["PITH_LOCAL_PLUGIN_DIR"] = AppSupportDirectories.pluginInstallDirectory().path
    applyBundleResourceEnvironment(to: &environment)
    if let activeModel = await activeLocalModelSelection() {
      environment["PITH_MODEL_PACK_MANIFEST"] = activeModel.manifestPath
      environment["PITH_MODEL_PATH"] = activeModel.modelPath
      environment["PITH_LFM_MODEL_PATH"] = activeModel.modelPath
    }
    return environment
  }

  private static func sanitizedInheritedEnvironment() -> [String: String] {
    var environment = ProcessInfo.processInfo.environment
    let appOwnedKeys = environment.keys.filter { $0.hasPrefix(appEnvironmentPrefix) }
    for key in appOwnedKeys {
      environment.removeValue(forKey: key)
    }
    return environment
  }

  private static func applyBundleResourceEnvironment(to environment: inout [String: String]) {
    guard let resourceURL = Bundle.main.resourceURL else {
      return
    }

    if shouldRequirePackagedModelBackend() {
      environment["PITH_REQUIRE_PACKAGED_LLAMACPP"] = "1"
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

  private static func shouldRequirePackagedModelBackend() -> Bool {
    Bundle.main.bundleURL.pathExtension == "app"
  }

  private static func activeLocalModelSelection() async -> RuntimeBridgeActiveLocalModelSelection? {
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
      let missingFileDetail =
        "The saved active local model was reset because its setup file or model file no longer "
        + "exists. Choose or download a model to continue."
      clearActiveLocalModel(
        invalidationDetail: missingFileDetail
      )
      return nil
    }

    guard await LocalModelCatalog.isVerifiedInstalledSelectionInBackground(
      storageRootPath: localModelStorageRootPath(),
      modelPath: modelPath,
      manifestPath: manifestPath
    ) else {
      let verificationDetail =
        "The saved active local model was reset because its setup file and model file no longer "
        + "match Pith's verified catalog. Choose or download a model to continue."
      clearActiveLocalModel(
        invalidationDetail: verificationDetail
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
