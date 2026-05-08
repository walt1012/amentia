import Foundation

enum LocalModelCatalog {
  static let defaultFirstUseModelID = "lfm2.5-350m"

  static func summaries(
    storageRootPath: String,
    activeModelPath: String?
  ) -> [LocalModelSummary] {
    let normalizedActivePath = activeModelPath.map { normalizedPath($0) }
    return items().map { item in
      let installPath = item.installPath(storageRootPath: storageRootPath)
      let normalizedInstallPath = normalizedPath(installPath)
      let localState = LocalModelIntegrity.state(at: installPath, item: item)
      return LocalModelSummary(
        id: item.id,
        displayName: item.displayName,
        description: item.description,
        fileName: item.fileName,
        downloadURL: item.downloadURL,
        homepage: item.homepage,
        sizeBytes: item.sizeBytes,
        sha256: item.sha256,
        contextSize: item.contextSize,
        modelContextSize: item.modelContextSize,
        maxOutputTokens: item.maxOutputTokens,
        license: item.license,
        tags: item.tags,
        installPath: installPath,
        downloaded: localState.isVerified,
        active: localState.isVerified && normalizedActivePath == Optional(normalizedInstallPath),
        localSizeBytes: localState.localSizeBytes
      )
    }
  }

  static func writePackManifest(for model: LocalModelSummary) throws -> String {
    let manifestURL = packManifestURL(forModelPath: model.installPath)
    let manifest = packManifest(for: model)
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    let data = try encoder.encode(manifest)
    try FileManager.default.createDirectory(
      at: manifestURL.deletingLastPathComponent(),
      withIntermediateDirectories: true
    )
    try data.write(to: manifestURL, options: .atomic)
    return manifestURL.path
  }

  static func validateDownloadedModel(_ model: LocalModelSummary) throws {
    try LocalModelIntegrity.validateDownloadedModel(model)
  }

  static func isVerifiedInstalledModel(
    storageRootPath: String,
    modelPath: String
  ) -> Bool {
    verifiedInstalledModel(storageRootPath: storageRootPath, modelPath: modelPath) != nil
  }

  static func isVerifiedInstalledSelection(
    storageRootPath: String,
    modelPath: String,
    manifestPath: String
  ) -> Bool {
    guard let model = verifiedInstalledModel(
      storageRootPath: storageRootPath,
      modelPath: modelPath
    ) else {
      return false
    }

    let manifestURL = packManifestURL(forModelPath: model.installPath)
    guard normalizedPath(manifestURL.path) == normalizedPath(manifestPath),
          FileManager.default.fileExists(atPath: manifestURL.path)
    else {
      return false
    }

    guard let manifest = try? packManifest(at: manifestURL) else {
      return false
    }

    return manifest == packManifest(for: model)
  }

  private static func verifiedInstalledModel(
    storageRootPath: String,
    modelPath: String
  ) -> LocalModelSummary? {
    let normalizedModelPath = normalizedPath(modelPath)
    return summaries(
      storageRootPath: storageRootPath,
      activeModelPath: modelPath
    )
    .first { model in
      model.downloaded && normalizedPath(model.installPath) == normalizedModelPath
    }
  }

  private static func packManifestURL(forModelPath modelPath: String) -> URL {
    URL(fileURLWithPath: modelPath)
      .deletingLastPathComponent()
      .appendingPathComponent("model-pack.json")
  }

  private static func packManifest(for model: LocalModelSummary) -> LocalModelPackManifest {
    LocalModelPackManifest(
      id: model.id,
      displayName: model.displayName,
      fileName: model.fileName,
      contextSize: model.contextSize,
      modelContextSize: model.modelContextSize,
      maxOutputTokens: model.maxOutputTokens,
      backend: "llama.cpp",
      license: model.license,
      homepage: model.homepage,
      downloadURL: model.downloadURL,
      sha256: model.sha256,
      sizeBytes: model.sizeBytes
    )
  }

  private static func packManifest(at manifestURL: URL) throws -> LocalModelPackManifest {
    let data = try Data(contentsOf: manifestURL)
    return try JSONDecoder().decode(LocalModelPackManifest.self, from: data)
  }

  private static func normalizedPath(_ path: String) -> String {
    URL(fileURLWithPath: path).standardizedFileURL.path
  }
}

private struct LocalModelPackManifest: Codable, Equatable {
  let id: String
  let displayName: String
  let fileName: String
  let contextSize: Int
  let modelContextSize: Int
  let maxOutputTokens: Int
  let backend: String
  let license: String
  let homepage: String
  let downloadURL: String
  let sha256: String
  let sizeBytes: Int64

  enum CodingKeys: String, CodingKey {
    case id
    case displayName = "display_name"
    case fileName = "file_name"
    case contextSize = "context_size"
    case modelContextSize = "model_context_size"
    case maxOutputTokens = "max_output_tokens"
    case backend
    case license
    case homepage
    case downloadURL = "download_url"
    case sha256
    case sizeBytes = "size_bytes"
  }
}
