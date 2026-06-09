import Foundation

struct ModelHealthSummary: Hashable, Sendable {
  let packID: String
  let displayName: String
  let backend: String
  let status: String
  let detail: String
  let source: String
  let binaryPath: String?
  let modelPath: String?
  let manifestPath: String?
  let metrics: [String: String]
}

struct RuntimeReadinessCheckSummary: Identifiable, Hashable, Sendable {
  let id: String
  let title: String
  let status: String
  let detail: String
}

struct RuntimeReadinessSummary: Hashable, Sendable {
  let status: String
  let summary: String
  let checks: [RuntimeReadinessCheckSummary]
  let metrics: [String: String]
}

struct LocalModelSummary: Identifiable, Hashable, Sendable {
  let id: String
  let displayName: String
  let description: String
  let fileName: String
  let downloadURL: String
  let homepage: String
  let sizeBytes: Int64
  let sha256: String
  let contextSize: Int
  let modelContextSize: Int
  let maxOutputTokens: Int
  let license: String
  let tags: [String]
  let installPath: String
  let downloaded: Bool
  let active: Bool
  let localSizeBytes: Int64?

  var hasLocalFile: Bool {
    localSizeBytes != nil
  }

  var needsVerification: Bool {
    hasLocalFile && !downloaded
  }
}
