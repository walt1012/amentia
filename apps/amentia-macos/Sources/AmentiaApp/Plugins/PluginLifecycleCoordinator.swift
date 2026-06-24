import Foundation

final class PluginLifecycleOperationCoordinator {
  private let taskSlot = CancellableTaskSlot()

  var isActive: Bool {
    taskSlot.isActive
  }

  func begin() -> UUID? {
    taskSlot.begin()
  }

  func bind(task: Task<Void, Never>, operationID: UUID) {
    taskSlot.bind(task: task, requestID: operationID)
  }

  func isCurrent(_ operationID: UUID) -> Bool {
    taskSlot.isCurrent(operationID)
  }

  func finish(_ operationID: UUID) {
    taskSlot.finish(operationID)
  }

  func cancel() {
    taskSlot.cancel()
  }
}

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
