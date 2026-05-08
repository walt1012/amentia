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
    let modelURL = URL(fileURLWithPath: model.installPath)
    let manifestURL = modelURL
      .deletingLastPathComponent()
      .appendingPathComponent("model-pack.json")
    let manifest = LocalModelPackManifest(
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
    let normalizedModelPath = normalizedPath(modelPath)
    return summaries(
      storageRootPath: storageRootPath,
      activeModelPath: modelPath
    )
    .contains { model in
      model.downloaded && normalizedPath(model.installPath) == normalizedModelPath
    }
  }

  private static func normalizedPath(_ path: String) -> String {
    URL(fileURLWithPath: path).standardizedFileURL.path
  }
}

private struct LocalModelPackManifest: Encodable {
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
