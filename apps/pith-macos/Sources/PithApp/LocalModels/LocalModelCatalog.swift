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
  private static let verifiedModelKeyPrefix = "pith.verifiedLocalModel."

  static func summaries(
    storageRootPath: String,
    activeModelPath: String?
  ) -> [LocalModelSummary] {
    let manager = FileManager.default
    let normalizedActivePath = activeModelPath.map { normalizedPath($0) }
    return items().map { item in
      let installPath = item.installPath(storageRootPath: storageRootPath)
      let normalizedInstallPath = normalizedPath(installPath)
      let localMetadata = localFileMetadata(at: installPath)
      let localSizeBytes = localMetadata?.sizeBytes
      let downloaded = manager.fileExists(atPath: installPath)
        && isVerifiedModelFile(at: installPath, localMetadata: localMetadata, item: item)
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
    let fileURL = URL(fileURLWithPath: model.installPath)
    let metadata = try localFileMetadataOrThrow(at: fileURL)
    try validateModelSize(
      metadata.sizeBytes,
      displayName: model.displayName,
      expectedSizeBytes: model.sizeBytes
    )
    try validateGGUFMagic(at: fileURL, displayName: model.displayName)

    try validateExpectedSHA256(model.sha256, displayName: model.displayName)
    let actualSHA256 = try sha256Hex(at: fileURL)
    guard actualSHA256.caseInsensitiveCompare(model.sha256) == .orderedSame else {
      throw LocalModelIntegrityError.checksumMismatch(
        displayName: model.displayName,
        expected: model.sha256,
        actual: actualSHA256
      )
    }

    rememberVerifiedModel(
      modelID: model.id,
      path: model.installPath,
      expectedSHA256: model.sha256,
      localMetadata: metadata
    )
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

  private static func localFileMetadata(at path: String) -> LocalModelFileMetadata? {
    guard let attributes = try? FileManager.default.attributesOfItem(atPath: path),
          let size = attributes[.size] as? NSNumber
    else {
      return nil
    }

    return LocalModelFileMetadata(
      sizeBytes: size.int64Value,
      modificationDate: attributes[.modificationDate] as? Date
    )
  }

  private static func localFileMetadataOrThrow(at fileURL: URL) throws -> LocalModelFileMetadata {
    let attributes = try FileManager.default.attributesOfItem(atPath: fileURL.path)
    guard let size = attributes[.size] as? NSNumber else {
      throw LocalModelIntegrityError.missingSize(path: fileURL.path)
    }

    return LocalModelFileMetadata(
      sizeBytes: size.int64Value,
      modificationDate: attributes[.modificationDate] as? Date
    )
  }

  private static func isVerifiedModelFile(
    at path: String,
    localMetadata: LocalModelFileMetadata?,
    item: LocalModelCatalogItem
  ) -> Bool {
    guard let localMetadata else {
      return false
    }

    do {
      try validateModelSize(
        localMetadata.sizeBytes,
        displayName: item.displayName,
        expectedSizeBytes: item.sizeBytes
      )
      try validateGGUFMagic(at: URL(fileURLWithPath: path), displayName: item.displayName)
      try validateExpectedSHA256(item.sha256, displayName: item.displayName)
      if hasVerifiedModel(
        modelID: item.id,
        path: path,
        expectedSHA256: item.sha256,
        localMetadata: localMetadata
      ) {
        return true
      }

      let actualSHA256 = try sha256Hex(at: URL(fileURLWithPath: path))
      guard actualSHA256.caseInsensitiveCompare(item.sha256) == .orderedSame else {
        forgetVerifiedModel(modelID: item.id)
        return false
      }
      rememberVerifiedModel(
        modelID: item.id,
        path: path,
        expectedSHA256: item.sha256,
        localMetadata: localMetadata
      )
      return true
    } catch {
      forgetVerifiedModel(modelID: item.id)
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

  private static func validateExpectedSHA256(
    _ expectedSHA256: String,
    displayName: String
  ) throws {
    guard !expectedSHA256.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      throw LocalModelIntegrityError.missingChecksum(displayName: displayName)
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

  private static func hasVerifiedModel(
    modelID: String,
    path: String,
    expectedSHA256: String,
    localMetadata: LocalModelFileMetadata
  ) -> Bool {
    UserDefaults.standard.string(forKey: verifiedModelKey(for: modelID)) == verificationStamp(
      path: path,
      expectedSHA256: expectedSHA256,
      localMetadata: localMetadata
    )
  }

  private static func rememberVerifiedModel(
    modelID: String,
    path: String,
    expectedSHA256: String,
    localMetadata: LocalModelFileMetadata
  ) {
    let stamp = verificationStamp(
      path: path,
      expectedSHA256: expectedSHA256,
      localMetadata: localMetadata
    )
    UserDefaults.standard.set(stamp, forKey: verifiedModelKey(for: modelID))
  }

  private static func forgetVerifiedModel(modelID: String) {
    UserDefaults.standard.removeObject(forKey: verifiedModelKey(for: modelID))
  }

  private static func verifiedModelKey(for modelID: String) -> String {
    "\(verifiedModelKeyPrefix)\(modelID)"
  }

  private static func verificationStamp(
    path: String,
    expectedSHA256: String,
    localMetadata: LocalModelFileMetadata
  ) -> String {
    [
      normalizedPath(path),
      String(localMetadata.sizeBytes),
      String(localMetadata.modificationMilliseconds),
      expectedSHA256.lowercased(),
    ].joined(separator: "|")
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
}

private struct LocalModelFileMetadata {
  let sizeBytes: Int64
  let modificationDate: Date?

  var modificationMilliseconds: Int64 {
    guard let modificationDate else {
      return 0
    }

    return Int64(modificationDate.timeIntervalSince1970 * 1000)
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

private enum LocalModelIntegrityError: LocalizedError {
  case missingSize(path: String)
  case sizeTooSmall(displayName: String, expectedMinimumBytes: Int64, actualBytes: Int64)
  case invalidMagic(displayName: String)
  case missingChecksum(displayName: String)
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
    case .missingChecksum(let displayName):
      return "\(displayName) is missing required SHA-256 metadata."
    case .checksumMismatch(let displayName, let expected, let actual):
      return "\(displayName) checksum mismatch. Expected \(expected), found \(actual)."
    }
  }
}
