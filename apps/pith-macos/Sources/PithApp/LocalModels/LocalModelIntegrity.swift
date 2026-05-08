import CryptoKit
import Foundation

struct LocalModelIntegrityState {
  let localSizeBytes: Int64?
  let isVerified: Bool
}

enum LocalModelIntegrity {
  private static let ggufMagic = Data([0x47, 0x47, 0x55, 0x46])
  private static let minimumModelBytes: Int64 = 64 * 1024 * 1024
  private static let verifiedModelKeyPrefix = "pith.verifiedLocalModel."

  static func state(at path: String, item: LocalModelCatalogItem) -> LocalModelIntegrityState {
    let localMetadata = localFileMetadata(at: path)
    let isVerified = FileManager.default.fileExists(atPath: path)
      && isVerifiedModelFile(at: path, localMetadata: localMetadata, item: item)
    return LocalModelIntegrityState(
      localSizeBytes: localMetadata?.sizeBytes,
      isVerified: isVerified
    )
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
