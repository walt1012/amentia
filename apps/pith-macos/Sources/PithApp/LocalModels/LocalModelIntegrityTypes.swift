import Foundation

struct LocalModelFileMetadata {
  let sizeBytes: Int64
  let modificationDate: Date?

  var modificationMilliseconds: Int64 {
    guard let modificationDate else {
      return 0
    }

    return Int64(modificationDate.timeIntervalSince1970 * 1000)
  }
}

enum LocalModelIntegrityError: LocalizedError {
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
