import Foundation

enum PluginLifecycleCoordinator {
  static func refreshState(using runtimeBridge: RuntimeBridge) async -> PluginStateRefresh {
    await PluginStateLoader.refresh(using: runtimeBridge)
  }

  static func inspectInstallSource(
    _ sourceURL: URL,
    runtimeBridge: RuntimeBridge,
    installRootPath: String
  ) async throws -> PluginInstallPreview {
    let inspection = try await runtimeBridge.inspectPlugin(sourcePath: sourceURL.path)
    return PluginInstallInspector.preview(
      for: sourceURL,
      inspection: inspection,
      installRootPath: installRootPath
    )
  }

  static func installConfirmedPlugin(
    preview: PluginInstallPreview,
    runtimeBridge: RuntimeBridge
  ) async throws -> RuntimeBridge.RuntimePlugin {
    try await runtimeBridge.installPlugin(sourcePath: preview.sourcePath)
  }

  static func setPluginEnabled(
    pluginID: String,
    enabled: Bool,
    runtimeBridge: RuntimeBridge
  ) async throws -> RuntimeBridge.RuntimePlugin {
    try await runtimeBridge.setPluginEnabled(pluginID: pluginID, enabled: enabled)
  }

  static func removePlugin(
    plugin: PluginSummary,
    runtimeBridge: RuntimeBridge
  ) async throws -> RuntimeBridge.RuntimePluginRemoval {
    try await runtimeBridge.removePlugin(manifestPath: plugin.manifestPath)
  }
}
