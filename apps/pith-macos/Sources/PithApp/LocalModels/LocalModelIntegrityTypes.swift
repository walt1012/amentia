import Foundation

struct LocalModelFileMetadata {
  let sizeBytes: Int64
  let creationDate: Date?
  let modificationDate: Date?
  let systemFileNumber: UInt64?

  var creationMilliseconds: Int64 {
    guard let creationDate else {
      return 0
    }

    return Int64(creationDate.timeIntervalSince1970 * 1000)
  }

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
    case .missingSize:
      return "Pith could not inspect this local model file. Try downloading it again."
    case .sizeTooSmall(let displayName, _, _):
      return "\(displayName) is incomplete. Cancel or replace it with a fresh download."
    case .invalidMagic(let displayName):
      return "\(displayName) is not a valid local model file."
    case .missingChecksum(let displayName):
      return "\(displayName) is missing verification metadata."
    case .checksumMismatch(let displayName, _, _):
      return "\(displayName) did not match the verified download. Replace it with a fresh download."
    }
  }
}
