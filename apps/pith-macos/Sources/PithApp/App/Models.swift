import Foundation

struct ThreadSummary: Identifiable, Hashable {
  let id: String
  var title: String
  var preview: String
  var workspaceRootPath: String?
  var workspaceDisplayName: String?
}

struct WorkspaceSummary: Hashable {
  let rootPath: String
  let displayName: String
}

struct WorkspaceSearchMatchSummary: Identifiable, Hashable {
  let id: String
  let relativePath: String
  let lineNumber: Int
  let line: String
}

struct ModelHealthSummary: Hashable {
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

struct RuntimeReadinessCheckSummary: Identifiable, Hashable {
  let id: String
  let title: String
  let status: String
  let detail: String
}

struct RuntimeReadinessSummary: Hashable {
  let status: String
  let summary: String
  let checks: [RuntimeReadinessCheckSummary]
  let metrics: [String: String]
}

struct LocalModelSummary: Identifiable, Hashable {
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
}

struct MemoryStatusSummary: Hashable {
  let noteCount: Int
  let latestTitle: String?
  let summary: String
}

struct MemoryNoteSummary: Identifiable, Hashable {
  let id: String
  let title: String
  let body: String
  let scope: String
  let source: String
  let createdAt: Int
  let tags: [String]
}

struct TimelineEntry: Identifiable, Hashable {
  enum Kind: String {
    case userMessage
    case assistantMessage
    case system
    case plan
    case tool
    case diff
    case approval
    case warning
  }

  let id: String
  let kind: Kind
  let title: String
  let body: String
  let attributes: [String: String]
}

enum DiffLineKind: String, Hashable {
  case addition
  case deletion
  case hunk
  case metadata
  case context
}

enum StatusTone: String, Hashable {
  case neutral
  case ready
  case active
  case warning
  case danger
}

struct ReadinessStepSummary: Identifiable, Hashable {
  let id: String
  let label: String
  let detail: String
  let tone: StatusTone
}

struct ComposerSuggestionSummary: Identifiable, Hashable {
  let id: String
  let title: String
  let message: String
}

struct DiffLineSummary: Identifiable, Hashable {
  let id: String
  let lineNumber: Int
  let text: String
  let kind: DiffLineKind
}
