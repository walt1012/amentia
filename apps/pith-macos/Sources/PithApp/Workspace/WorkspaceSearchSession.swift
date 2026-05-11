import Foundation

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

struct WorkspaceSearchSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let hasWorkspace: Bool
  let isSearching: Bool
  let query: String
}

struct WorkspaceSearchRequestToken {
  let id: UUID
  let query: String
  let status: String
}

struct WorkspaceSearchRuntimeState {
  var query: String
  var results: [WorkspaceSearchMatchSummary]
  var status: String
  var isSearching: Bool

  init(
    query: String = "",
    results: [WorkspaceSearchMatchSummary] = [],
    status: String = "Search the open workspace by text.",
    isSearching: Bool = false
  ) {
    self.query = query
    self.results = results
    self.status = status
    self.isSearching = isSearching
  }

  mutating func begin(_ token: WorkspaceSearchRequestToken) {
    results = []
    isSearching = true
    status = token.status
  }

  mutating func finishWithChangedQuery(status: String) {
    results = []
    self.status = status
    isSearching = false
  }

  mutating func finishWithMatches(
    _ matches: [WorkspaceSearchMatchSummary],
    status: String
  ) {
    results = matches
    self.status = status
    isSearching = false
  }

  mutating func finishWithFailure(status: String) {
    results = []
    self.status = status
    isSearching = false
  }

  mutating func reset(status: String) {
    results = []
    self.status = status
    isSearching = false
  }
}

final class WorkspaceSearchSession {
  private let taskSlot = CancellableTaskSlot()
  private var activeQuery: String?

  static func trimmedQuery(_ query: String) -> String {
    query.trimmingCharacters(in: .whitespacesAndNewlines)
  }

  static func canSearch(_ snapshot: WorkspaceSearchSnapshot) -> Bool {
    snapshot.runtimeState == .ready
      && snapshot.hasWorkspace
      && !trimmedQuery(snapshot.query).isEmpty
  }

  static func queryChanged(currentQuery: String, token: WorkspaceSearchRequestToken) -> Bool {
    trimmedQuery(currentQuery) != token.query
  }

  static func matchSummaries(
    from matches: [RuntimeBridge.RuntimeWorkspaceSearchMatch]
  ) -> [WorkspaceSearchMatchSummary] {
    matches.enumerated().map { index, match in
      WorkspaceSearchMatchSummary(
        id: "\(match.relativePath):\(match.lineNumber):\(index)",
        relativePath: match.relativePath,
        lineNumber: match.lineNumber,
        line: match.line
      )
    }
  }

  func begin(query: String) -> WorkspaceSearchRequestToken {
    let requestID = taskSlot.replace()
    activeQuery = query
    return WorkspaceSearchRequestToken(
      id: requestID,
      query: query,
      status: "Searching for \"\(query)\"..."
    )
  }

  func canStart(query: String) -> Bool {
    activeQuery != query
  }

  func bind(task: Task<Void, Never>, token: WorkspaceSearchRequestToken) {
    taskSlot.bind(task: task, requestID: token.id)
  }

  func isCurrent(_ token: WorkspaceSearchRequestToken) -> Bool {
    taskSlot.isCurrent(token.id)
  }

  func finish(_ token: WorkspaceSearchRequestToken) {
    guard isCurrent(token) else {
      return
    }

    clearActiveSearch()
  }

  func resetStatus(hasWorkspace: Bool) -> String {
    cancelActiveSearch()
    return hasWorkspace
      ? "Search the open workspace by text."
      : "Open a workspace before searching."
  }

  func changedQueryStatus() -> String {
    clearActiveSearch()
    return "Query changed. Press Return to search again."
  }

  private func cancelActiveSearch() {
    taskSlot.cancel()
    activeQuery = nil
  }

  private func clearActiveSearch() {
    activeQuery = nil
    taskSlot.clear()
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
