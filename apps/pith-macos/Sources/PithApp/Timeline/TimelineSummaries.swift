import Foundation

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
