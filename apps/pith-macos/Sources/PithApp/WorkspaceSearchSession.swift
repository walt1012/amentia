import Foundation

struct WorkspaceSearchRequestToken {
  let id: UUID
  let query: String
  let status: String
}

final class WorkspaceSearchSession {
  private var activeRequestID: UUID?

  func begin(query: String) -> WorkspaceSearchRequestToken {
    let requestID = UUID()
    activeRequestID = requestID
    return WorkspaceSearchRequestToken(
      id: requestID,
      query: query,
      status: "Searching for \"\(query)\"..."
    )
  }

  func isCurrent(_ token: WorkspaceSearchRequestToken) -> Bool {
    activeRequestID == token.id
  }

  func finish() {
    activeRequestID = nil
  }

  func resetStatus(hasWorkspace: Bool) -> String {
    activeRequestID = nil
    return hasWorkspace
      ? "Search the open workspace by text."
      : "Open a workspace before searching."
  }

  func changedQueryStatus() -> String {
    activeRequestID = nil
    return "Query changed. Press Return to search again."
  }

  static func successStatus(query: String, matchCount: Int) -> String {
    matchCount == 0
      ? "No matches found for \"\(query)\"."
      : "Found \(matchCount) match(es) for \"\(query)\"."
  }

  static func failureStatus(error: Error) -> String {
    "Workspace search failed: \(error.localizedDescription)"
  }

  static func emptyStateSummary(
    runtimeState: RuntimeBridge.ConnectionState,
    hasWorkspace: Bool,
    query: String,
    status: String,
    isSearching: Bool,
    hasResults: Bool
  ) -> String? {
    if isSearching || hasResults {
      return nil
    }
    if runtimeState != .ready {
      return "Launch the runtime to search workspace files."
    }
    if !hasWorkspace {
      return "Open a workspace to search local files."
    }
    if query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
      return "Search file contents or symbols, then press Return."
    }
    if status.hasPrefix("No matches found") {
      return "No results yet. Try a shorter query, filename, or symbol name."
    }
    if status.hasPrefix("Workspace search failed") {
      return "Search failed. Check the runtime status, then try again."
    }

    return nil
  }

  static func overflowSummary(resultCount: Int) -> String? {
    guard resultCount > 8 else {
      return nil
    }

    return "Showing the first 8 matches. Narrow the query to focus the review."
  }
}
