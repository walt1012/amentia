import Foundation

enum LocalModelVerificationStampStore {
  private static let verifiedModelKeyPrefix = "pith.verifiedLocalModel."

  static func hasVerifiedModel(
    modelID: String,
    path: String,
    expectedSHA256: String,
    localMetadata: LocalModelFileMetadata
  ) -> Bool {
    guard let storedStamp = UserDefaults.standard.string(forKey: verifiedModelKey(for: modelID)) else {
      return false
    }

    let currentStamp = verificationStamp(
      path: path,
      expectedSHA256: expectedSHA256,
      localMetadata: localMetadata
    )
    if storedStamp == currentStamp {
      return true
    }

    return storedStamp == legacyVerificationStamp(
      path: path,
      expectedSHA256: expectedSHA256,
      localMetadata: localMetadata
    )
  }

  static func rememberVerifiedModel(
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

  static func forgetVerifiedModel(modelID: String) {
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
      String(localMetadata.creationMilliseconds),
      String(localMetadata.modificationMilliseconds),
      String(localMetadata.systemFileNumber ?? 0),
      expectedSHA256.lowercased(),
    ].joined(separator: "|")
  }

  private static func legacyVerificationStamp(
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
