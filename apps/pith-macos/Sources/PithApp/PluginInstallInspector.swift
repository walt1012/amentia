import Foundation

struct LocalPluginAuthor: Decodable {
  let name: String
}

struct LocalPluginManifest: Decodable {
  let name: String
  let version: String
  let displayName: String
  let description: String
  let author: LocalPluginAuthor?
  let capabilities: [String]
  let permissions: [String]
  let defaultEnabled: Bool
}

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
  static func preview(for url: URL, installRootPath: String) throws -> PluginInstallPreview {
    let manifestURL = try pluginManifestURL(for: url)
    let data = try Data(contentsOf: manifestURL)
    let manifest = try JSONDecoder().decode(LocalPluginManifest.self, from: data)
    try validatePluginName(manifest.name)

    let installRoot = URL(fileURLWithPath: installRootPath, isDirectory: true)
    let installURL = installRoot.appendingPathComponent(manifest.name, isDirectory: true)

    return PluginInstallPreview(
      sourcePath: url.path,
      manifestPath: manifestURL.path,
      installPath: installURL.path,
      displayName: manifest.displayName,
      version: manifest.version,
      description: manifest.description,
      authorName: manifest.author?.name,
      capabilities: manifest.capabilities,
      permissions: manifest.permissions,
      defaultEnabled: manifest.defaultEnabled
    )
  }

  private static func pluginManifestURL(for url: URL) throws -> URL {
    var isDirectory = ObjCBool(false)
    if FileManager.default.fileExists(atPath: url.path, isDirectory: &isDirectory),
       isDirectory.boolValue
    {
      let manifestURL = url.appendingPathComponent("pith-plugin.json", isDirectory: false)
      guard FileManager.default.fileExists(atPath: manifestURL.path) else {
        throw NSError(
          domain: "PithPluginInstall",
          code: 1,
          userInfo: [
            NSLocalizedDescriptionKey:
              "The selected folder does not contain pith-plugin.json."
          ]
        )
      }
      return manifestURL
    }

    guard url.lastPathComponent == "pith-plugin.json" else {
      throw NSError(
        domain: "PithPluginInstall",
        code: 2,
        userInfo: [
          NSLocalizedDescriptionKey:
            "Select a plugin folder or a pith-plugin.json manifest."
        ]
      )
    }

    return url
  }

  private static func validatePluginName(_ name: String) throws {
    guard !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      throw pluginNameError("Plugin manifest name must not be empty.")
    }

    guard name != "." && name != ".." else {
      throw pluginNameError("Plugin manifest name must not be a relative path segment.")
    }

    guard !name.contains("/") && !name.contains("\\") && !name.contains(":") else {
      throw pluginNameError("Plugin manifest name must not contain path separators or colons.")
    }
  }

  private static func pluginNameError(_ message: String) -> NSError {
    NSError(
      domain: "PithPluginInstall",
      code: 3,
      userInfo: [
        NSLocalizedDescriptionKey: message
      ]
    )
  }
}
