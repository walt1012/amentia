import CryptoKit
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

  private static let ggufMagic = Data([0x47, 0x47, 0x55, 0x46])
  private static let minimumModelBytes: Int64 = 64 * 1024 * 1024
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
      let localSizeBytes = localFileSize(at: installPath)
      let downloaded = manager.fileExists(atPath: installPath)
        && isPlausibleModelFile(at: installPath, localSizeBytes: localSizeBytes, item: item)
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
        maxOutputTokens: item.maxOutputTokens,
        license: item.license,
        tags: item.tags,
        installPath: installPath,
        downloaded: downloaded,
        active: downloaded && normalizedActivePath == Optional(normalizedInstallPath),
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
    let fileURL = URL(fileURLWithPath: model.installPath)
    let size = try localFileSizeOrThrow(at: fileURL)
    try validateModelSize(size, displayName: model.displayName, expectedSizeBytes: model.sizeBytes)
    try validateGGUFMagic(at: fileURL, displayName: model.displayName)

    if let expectedSHA256 = model.sha256, !expectedSHA256.isEmpty {
      let actualSHA256 = try sha256Hex(at: fileURL)
      guard actualSHA256.caseInsensitiveCompare(expectedSHA256) == .orderedSame else {
        throw LocalModelIntegrityError.checksumMismatch(
          displayName: model.displayName,
          expected: expectedSHA256,
          actual: actualSHA256
        )
      }
    }
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

  private static func localFileSizeOrThrow(at fileURL: URL) throws -> Int64 {
    let attributes = try FileManager.default.attributesOfItem(atPath: fileURL.path)
    guard let size = attributes[.size] as? NSNumber else {
      throw LocalModelIntegrityError.missingSize(path: fileURL.path)
    }

    return size.int64Value
  }

  private static func isPlausibleModelFile(
    at path: String,
    localSizeBytes: Int64?,
    item: LocalModelCatalogItem
  ) -> Bool {
    guard let localSizeBytes else {
      return false
    }

    do {
      try validateModelSize(
        localSizeBytes,
        displayName: item.displayName,
        expectedSizeBytes: item.sizeBytes
      )
      try validateGGUFMagic(at: URL(fileURLWithPath: path), displayName: item.displayName)
      return true
    } catch {
      return false
    }
  }

  private static func validateModelSize(
    _ localSizeBytes: Int64,
    displayName: String,
    expectedSizeBytes: Int64
  ) throws {
    let minimumBytes = max(minimumModelBytes, expectedSizeBytes * 9 / 10)
    guard localSizeBytes >= minimumBytes else {
      throw LocalModelIntegrityError.sizeTooSmall(
        displayName: displayName,
        expectedMinimumBytes: minimumBytes,
        actualBytes: localSizeBytes
      )
    }
  }

  private static func validateGGUFMagic(at fileURL: URL, displayName: String) throws {
    let handle = try FileHandle(forReadingFrom: fileURL)
    defer {
      try? handle.close()
    }

    let magic = try handle.read(upToCount: ggufMagic.count) ?? Data()
    guard magic == ggufMagic else {
      throw LocalModelIntegrityError.invalidMagic(displayName: displayName)
    }
  }

  private static func sha256Hex(at fileURL: URL) throws -> String {
    let handle = try FileHandle(forReadingFrom: fileURL)
    defer {
      try? handle.close()
    }

    var hasher = SHA256()
    while true {
      let chunk = try handle.read(upToCount: 1024 * 1024) ?? Data()
      if chunk.isEmpty {
        break
      }
      hasher.update(data: chunk)
    }

    return hasher.finalize().map { String(format: "%02x", $0) }.joined()
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
        sha256: "7e6f72643caafc9a68256686638c4d7916f2cec76d1df478d4c3ddcd95a6aed4",
        contextSize: 4096,
        maxOutputTokens: 160,
        license: "lfm1.0",
        tags: ["default", "tiny", "edge"],
        installSegments: ["builtin", defaultFirstUseModelID]
      ),
      LocalModelCatalogItem(
        id: "granite-4.0-h-350m",
        displayName: "Granite 4.0-H-350M Q4_K_M",
        description: "Modern Apache-2.0 tiny model for local tool, code, and RAG workflows.",
        fileName: "granite-4.0-h-350m-Q4_K_M.gguf",
        downloadURL: "https://huggingface.co/ibm-granite/granite-4.0-h-350m-GGUF/resolve/main/granite-4.0-h-350m-Q4_K_M.gguf",
        homepage: "https://huggingface.co/ibm-granite/granite-4.0-h-350m-GGUF",
        sizeBytes: 222_662_560,
        sha256: "0a8d6a7373602fadfba274a640ba784b86cc6847f1c67f1b0a90fa2ec266b7fb",
        contextSize: 4096,
        maxOutputTokens: 192,
        license: "apache-2.0",
        tags: ["recommended", "tiny", "tools", "code"],
        installSegments: ["catalog", "granite-4.0-h-350m"]
      ),
      LocalModelCatalogItem(
        id: "qwen3-0.6b",
        displayName: "Qwen3-0.6B Q8_0",
        description: "Newer small general model for stronger local agent and multilingual tasks.",
        fileName: "Qwen3-0.6B-Q8_0.gguf",
        downloadURL: "https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/resolve/main/Qwen3-0.6B-Q8_0.gguf",
        homepage: "https://huggingface.co/Qwen/Qwen3-0.6B-GGUF",
        sizeBytes: 639_446_688,
        sha256: "9465e63a22add5354d9bb4b99e90117043c7124007664907259bd16d043bb031",
        contextSize: 8192,
        maxOutputTokens: 256,
        license: "apache-2.0",
        tags: ["stronger", "agent", "multilingual", "qwen"],
        installSegments: ["catalog", "qwen3-0.6b"]
      ),
      LocalModelCatalogItem(
        id: "qwen2.5-coder-0.5b-instruct",
        displayName: "Qwen2.5-Coder-0.5B Q4_K_M",
        description: "Focused small code model kept as a lightweight coding fallback.",
        fileName: "qwen2.5-coder-0.5b-instruct-q4_k_m.gguf",
        downloadURL: "https://huggingface.co/Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF/resolve/main/qwen2.5-coder-0.5b-instruct-q4_k_m.gguf",
        homepage: "https://huggingface.co/Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF",
        sizeBytes: 491_400_064,
        sha256: "1d9614638d18024d0fbb36575a15f1302a3adf044df10345688ec4f6e1c4ff32",
        contextSize: 4096,
        maxOutputTokens: 192,
        license: "apache-2.0",
        tags: ["code", "fallback", "qwen"],
        installSegments: ["catalog", "qwen2.5-coder-0.5b-instruct"]
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
  let sha256: String?
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
  let sha256: String?
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
    case sha256
    case sizeBytes = "size_bytes"
  }
}

private enum LocalModelIntegrityError: LocalizedError {
  case missingSize(path: String)
  case sizeTooSmall(displayName: String, expectedMinimumBytes: Int64, actualBytes: Int64)
  case invalidMagic(displayName: String)
  case checksumMismatch(displayName: String, expected: String, actual: String)

  var errorDescription: String? {
    switch self {
    case .missingSize(let path):
      return "Could not inspect local model size at \(path)."
    case .sizeTooSmall(let displayName, let expectedMinimumBytes, let actualBytes):
      return
        "\(displayName) is incomplete. Expected at least \(expectedMinimumBytes) bytes, found \(actualBytes)."
    case .invalidMagic(let displayName):
      return "\(displayName) is not a valid GGUF file."
    case .checksumMismatch(let displayName, let expected, let actual):
      return "\(displayName) checksum mismatch. Expected \(expected), found \(actual)."
    }
  }
}
