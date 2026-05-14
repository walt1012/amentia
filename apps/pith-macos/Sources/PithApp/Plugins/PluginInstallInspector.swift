import Foundation

struct PluginInstallPreview {
  let pluginID: String
  let sourcePath: String
  let manifestPath: String
  let installPath: String
  let displayName: String
  let version: String
  let description: String
  let authorName: String?
  let capabilities: [String]
  let permissions: [String]
  let defaultEnabled: Bool
  let installStatus: String
  let installBlocker: String?
  let installRepairHint: String?

  var canInstall: Bool {
    installStatus == "ready" && installBlocker == nil
  }
}

enum PluginInstallInspector {
  static func preview(
    for url: URL,
    inspection: RuntimeBridge.RuntimePluginInspection,
    installRootPath: String
  ) -> PluginInstallPreview {
    let inspectedPlugin = inspection.plugin
    let installRoot = URL(fileURLWithPath: installRootPath, isDirectory: true)
    let installURL = installRoot.appendingPathComponent(inspectedPlugin.name, isDirectory: true)

    return PluginInstallPreview(
      pluginID: inspectedPlugin.id,
      sourcePath: url.path,
      manifestPath: inspectedPlugin.manifestPath,
      installPath: installURL.path,
      displayName: inspectedPlugin.displayName,
      version: inspectedPlugin.version,
      description: inspectedPlugin.description,
      authorName: inspectedPlugin.authorName,
      capabilities: inspectedPlugin.capabilities,
      permissions: inspectedPlugin.permissions,
      defaultEnabled: inspectedPlugin.defaultEnabled,
      installStatus: inspection.installStatus,
      installBlocker: inspection.installBlocker,
      installRepairHint: inspection.installRepairHint
    )
  }
}
