import Foundation

struct ThreadSummary: Identifiable, Hashable {
  let id: String
  var title: String
  var preview: String
}

struct WorkspaceSummary: Hashable {
  let rootPath: String
  let displayName: String
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

  let id: UUID
  let kind: Kind
  let title: String
  let body: String
  let attributes: [String: String]
}
