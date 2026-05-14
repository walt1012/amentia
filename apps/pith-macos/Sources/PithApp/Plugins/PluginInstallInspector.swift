import Foundation

struct PluginInstallPreview {
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
}

enum PluginInstallInspector {
  static func preview(
    for url: URL,
    inspectedPlugin: RuntimeBridge.RuntimePlugin,
    installRootPath: String
  ) -> PluginInstallPreview {
    let installRoot = URL(fileURLWithPath: installRootPath, isDirectory: true)
    let installURL = installRoot.appendingPathComponent(inspectedPlugin.name, isDirectory: true)

    return PluginInstallPreview(
      sourcePath: url.path,
      manifestPath: inspectedPlugin.manifestPath,
      installPath: installURL.path,
      displayName: inspectedPlugin.displayName,
      version: inspectedPlugin.version,
      description: inspectedPlugin.description,
      authorName: inspectedPlugin.authorName,
      capabilities: inspectedPlugin.capabilities,
      permissions: inspectedPlugin.permissions,
      defaultEnabled: inspectedPlugin.defaultEnabled
    )
  }
}
