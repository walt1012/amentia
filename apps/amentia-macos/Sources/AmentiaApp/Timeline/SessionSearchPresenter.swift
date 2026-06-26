import Foundation

enum SessionSearchPresenter {
  static let emptyMatchDetail = "Try a session title, recent request, or project name."

  static func filteredSessions(
    _ sessions: [ThreadSummary],
    query rawQuery: String
  ) -> [ThreadSummary] {
    let query = normalizedSearchText(rawQuery)
    guard !query.isEmpty else {
      return sessions
    }

    let queryTerms = query.split(separator: " ").map(String.init)
    return sessions.filter { session in
      let haystack = searchableText(for: session)
      return queryTerms.allSatisfy { haystack.contains($0) }
    }
  }

  private static func searchableText(for session: ThreadSummary) -> String {
    [
      session.title,
      session.preview,
      session.workspaceDisplayName,
      workspaceFolderName(from: session.workspaceRootPath),
    ]
    .compactMap { $0 }
    .map(normalizedSearchText)
    .joined(separator: " ")
  }

  private static func workspaceFolderName(from rootPath: String?) -> String? {
    guard let rootPath,
          !rootPath.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    else {
      return nil
    }

    return URL(fileURLWithPath: rootPath).lastPathComponent
  }

  private static func normalizedSearchText(_ text: String) -> String {
    text
      .folding(options: [.caseInsensitive, .diacriticInsensitive], locale: .current)
      .lowercased()
      .trimmingCharacters(in: .whitespacesAndNewlines)
      .components(separatedBy: .whitespacesAndNewlines)
      .filter { !$0.isEmpty }
      .joined(separator: " ")
  }
}
