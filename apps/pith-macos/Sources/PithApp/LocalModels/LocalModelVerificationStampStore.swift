import Foundation

enum LocalModelVerificationStampStore {
  private static let verifiedModelKeyPrefix = "pith.verifiedLocalModel."

  static func hasVerifiedModel(
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
      String(localMetadata.modificationMilliseconds),
      expectedSHA256.lowercased(),
    ].joined(separator: "|")
  }

  private static func normalizedPath(_ path: String) -> String {
    URL(fileURLWithPath: path).standardizedFileURL.path
  }
}
