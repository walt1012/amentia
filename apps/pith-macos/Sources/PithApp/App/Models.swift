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
