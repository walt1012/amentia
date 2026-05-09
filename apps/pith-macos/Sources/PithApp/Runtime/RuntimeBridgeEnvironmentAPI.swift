import Foundation

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
}
