import Foundation

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
