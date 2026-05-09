import Foundation

struct ThreadSummary: Identifiable, Hashable {
  let id: String
  var title: String
  var preview: String
  var workspaceRootPath: String?
  var workspaceDisplayName: String?
}
