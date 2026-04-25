import Foundation

struct ModelDownloadProgress: Hashable {
  let modelID: String
  let displayName: String
  var bytesReceived: Int64
  var totalBytes: Int64
  let startedAt: Date
  var updatedAt: Date
  let isResuming: Bool
}

struct PersistedModelDownload {
  let modelID: String
  let resumeData: Data
  let bytesReceived: Int64
  let totalBytes: Int64
  let updatedAt: Date
}

enum LocalModelCatalog {
  static let defaultFirstUseModelID = "lfm2.5-350m"

  private static let pausedDownloadIDKey = "pith.pausedModelDownloadID"
  private static let pausedDownloadBytesReceivedKey = "pith.pausedModelDownloadBytesReceived"
  private static let pausedDownloadTotalBytesKey = "pith.pausedModelDownloadTotalBytes"
  private static let pausedDownloadUpdatedAtKey = "pith.pausedModelDownloadUpdatedAt"

  static func summaries(
    storageRootPath: String,
    activeModelPath: String?
  ) -> [LocalModelSummary] {
    let manager = FileManager.default
    let normalizedActivePath = activeModelPath.map { normalizedPath($0) }
    return items().map { item in
      let installPath = item.installPath(storageRootPath: storageRootPath)
      let normalizedInstallPath = normalizedPath(installPath)
      let downloaded = manager.fileExists(atPath: installPath)
      let localSizeBytes = localFileSize(at: installPath)
      return LocalModelSummary(
        id: item.id,
        displayName: item.displayName,
        description: item.description,
        fileName: item.fileName,
        downloadURL: item.downloadURL,
        homepage: item.homepage,
        sizeBytes: item.sizeBytes,
        contextSize: item.contextSize,
        maxOutputTokens: item.maxOutputTokens,
        license: item.license,
        tags: item.tags,
        installPath: installPath,
        downloaded: downloaded,
        active: normalizedActivePath == Optional(normalizedInstallPath),
        localSizeBytes: localSizeBytes
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
      maxOutputTokens: model.maxOutputTokens,
      backend: "llama.cpp",
      license: model.license,
      homepage: model.homepage,
      downloadURL: model.downloadURL,
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

  static func loadPausedDownload(matching localModels: [LocalModelSummary]) -> PersistedModelDownload? {
    let defaults = UserDefaults.standard
    guard let modelID = defaults.string(forKey: pausedDownloadIDKey),
          localModels.contains(where: { $0.id == modelID }),
          let resumeData = try? Data(contentsOf: pausedDownloadResumeDataURL()),
          !resumeData.isEmpty
    else {
      clearPausedDownload()
      return nil
    }

    let bytesReceived = max(Int64(defaults.integer(forKey: pausedDownloadBytesReceivedKey)), 0)
    let totalBytes = max(Int64(defaults.integer(forKey: pausedDownloadTotalBytesKey)), 0)
    let updatedAt = defaults.object(forKey: pausedDownloadUpdatedAtKey) as? Date ?? Date()
    return PersistedModelDownload(
      modelID: modelID,
      resumeData: resumeData,
      bytesReceived: bytesReceived,
      totalBytes: totalBytes,
      updatedAt: updatedAt
    )
  }

  static func restoredProgress(
    from pausedDownload: PersistedModelDownload?,
    localModels: [LocalModelSummary]
  ) -> ModelDownloadProgress? {
    guard let pausedDownload,
          let model = localModels.first(where: { $0.id == pausedDownload.modelID })
    else {
      return nil
    }

    return ModelDownloadProgress(
      modelID: model.id,
      displayName: model.displayName,
      bytesReceived: pausedDownload.bytesReceived,
      totalBytes: pausedDownload.totalBytes > 0 ? pausedDownload.totalBytes : model.sizeBytes,
      startedAt: pausedDownload.updatedAt,
      updatedAt: pausedDownload.updatedAt,
      isResuming: true
    )
  }

  static func savePausedDownload(
    modelID: String,
    resumeData: Data,
    bytesReceived: Int64,
    totalBytes: Int64,
    updatedAt: Date
  ) {
    let resumeDataURL = pausedDownloadResumeDataURL()
    let manager = FileManager.default
    do {
      try manager.createDirectory(
        at: resumeDataURL.deletingLastPathComponent(),
        withIntermediateDirectories: true
      )
      try resumeData.write(to: resumeDataURL, options: .atomic)
    } catch {
      clearPausedDownload()
      return
    }

    let defaults = UserDefaults.standard
    defaults.set(modelID, forKey: pausedDownloadIDKey)
    defaults.set(max(bytesReceived, 0), forKey: pausedDownloadBytesReceivedKey)
    defaults.set(max(totalBytes, 0), forKey: pausedDownloadTotalBytesKey)
    defaults.set(updatedAt, forKey: pausedDownloadUpdatedAtKey)
  }

  static func clearPausedDownload() {
    try? FileManager.default.removeItem(at: pausedDownloadResumeDataURL())
    let defaults = UserDefaults.standard
    defaults.removeObject(forKey: pausedDownloadIDKey)
    defaults.removeObject(forKey: pausedDownloadBytesReceivedKey)
    defaults.removeObject(forKey: pausedDownloadTotalBytesKey)
    defaults.removeObject(forKey: pausedDownloadUpdatedAtKey)
  }

  private static func localFileSize(at path: String) -> Int64? {
    guard let attributes = try? FileManager.default.attributesOfItem(atPath: path),
          let size = attributes[.size] as? NSNumber
    else {
      return nil
    }

    return size.int64Value
  }

  private static func pausedDownloadResumeDataURL() -> URL {
    let baseDirectory =
      FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
      ?? URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)

    return baseDirectory
      .appendingPathComponent("Pith", isDirectory: true)
      .appendingPathComponent("model-downloads", isDirectory: true)
      .appendingPathComponent("resume.data")
  }

  private static func normalizedPath(_ path: String) -> String {
    URL(fileURLWithPath: path).standardizedFileURL.path
  }

  private static func items() -> [LocalModelCatalogItem] {
    [
      LocalModelCatalogItem(
        id: defaultFirstUseModelID,
        displayName: "LFM2.5-350M Q4_K_M",
        description: "Default tiny local model for the first Pith agent loop.",
        fileName: "LFM2.5-350M-Q4_K_M.gguf",
        downloadURL: "https://huggingface.co/LiquidAI/LFM2.5-350M-GGUF/resolve/main/LFM2.5-350M-Q4_K_M.gguf",
        homepage: "https://huggingface.co/LiquidAI/LFM2.5-350M-GGUF",
        sizeBytes: 229_312_224,
        contextSize: 4096,
        maxOutputTokens: 160,
        license: "lfm1.0",
        tags: ["default", "tiny", "edge"],
        installSegments: ["builtin", defaultFirstUseModelID]
      ),
      LocalModelCatalogItem(
        id: "qwen2.5-coder-0.5b-instruct",
        displayName: "Qwen2.5-Coder-0.5B Q4_K_M",
        description: "Small code-oriented model for local code generation and repair experiments.",
        fileName: "qwen2.5-coder-0.5b-instruct-q4_k_m.gguf",
        downloadURL: "https://huggingface.co/Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF/resolve/main/qwen2.5-coder-0.5b-instruct-q4_k_m.gguf",
        homepage: "https://huggingface.co/Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF",
        sizeBytes: 491_000_000,
        contextSize: 4096,
        maxOutputTokens: 192,
        license: "apache-2.0",
        tags: ["code", "0.5B", "qwen"],
        installSegments: ["catalog", "qwen2.5-coder-0.5b-instruct"]
      ),
      LocalModelCatalogItem(
        id: "qwen2.5-0.5b-instruct",
        displayName: "Qwen2.5-0.5B Instruct Q4_K_M",
        description: "Compact general chat model with strong multilingual coverage for its size.",
        fileName: "qwen2.5-0.5b-instruct-q4_k_m.gguf",
        downloadURL: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf",
        homepage: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF",
        sizeBytes: 491_000_000,
        contextSize: 4096,
        maxOutputTokens: 192,
        license: "apache-2.0",
        tags: ["chat", "0.5B", "multilingual"],
        installSegments: ["catalog", "qwen2.5-0.5b-instruct"]
      ),
      LocalModelCatalogItem(
        id: "smollm2-360m-instruct",
        displayName: "SmolLM2-360M Q4_K_M",
        description: "Very small instruction model for fast local English assistant experiments.",
        fileName: "SmolLM2-360M-Instruct.Q4_K_M.gguf",
        downloadURL: "https://huggingface.co/QuantFactory/SmolLM2-360M-Instruct-GGUF/resolve/main/SmolLM2-360M-Instruct.Q4_K_M.gguf",
        homepage: "https://huggingface.co/QuantFactory/SmolLM2-360M-Instruct-GGUF",
        sizeBytes: 271_000_000,
        contextSize: 4096,
        maxOutputTokens: 160,
        license: "apache-2.0",
        tags: ["tiny", "english", "fast"],
        installSegments: ["catalog", "smollm2-360m-instruct"]
      ),
    ]
  }
}

private struct LocalModelCatalogItem {
  let id: String
  let displayName: String
  let description: String
  let fileName: String
  let downloadURL: String
  let homepage: String
  let sizeBytes: Int64
  let contextSize: Int
  let maxOutputTokens: Int
  let license: String
  let tags: [String]
  let installSegments: [String]

  func installPath(storageRootPath: String) -> String {
    installSegments.reduce(URL(fileURLWithPath: storageRootPath, isDirectory: true)) { url, segment in
      url.appendingPathComponent(segment)
    }
    .appendingPathComponent(fileName)
    .path
  }
}

private struct LocalModelPackManifest: Encodable {
  let id: String
  let displayName: String
  let fileName: String
  let contextSize: Int
  let maxOutputTokens: Int
  let backend: String
  let license: String
  let homepage: String
  let downloadURL: String
  let sizeBytes: Int64

  enum CodingKeys: String, CodingKey {
    case id
    case displayName = "display_name"
    case fileName = "file_name"
    case contextSize = "context_size"
    case maxOutputTokens = "max_output_tokens"
    case backend
    case license
    case homepage
    case downloadURL = "download_url"
    case sizeBytes = "size_bytes"
  }
}
