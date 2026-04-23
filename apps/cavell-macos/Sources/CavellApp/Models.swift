import Foundation

struct ThreadSummary: Identifiable, Hashable {
  let id: String
  var title: String
  var preview: String
}

struct TimelineEntry: Identifiable, Hashable {
  enum Kind: String {
    case userMessage
    case assistantMessage
    case system
  }

  let id: UUID
  let kind: Kind
  let title: String
  let body: String
}
