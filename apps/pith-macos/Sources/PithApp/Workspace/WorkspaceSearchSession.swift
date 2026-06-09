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
  let runtimeRequestID: String
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
    status: String = "Search the open workspace for useful context.",
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
  private var activeRuntimeRequestID: String?

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
    let runtimeRequestID = requestID.uuidString
    activeQuery = query
    activeRuntimeRequestID = runtimeRequestID
    return WorkspaceSearchRequestToken(
      id: requestID,
      runtimeRequestID: runtimeRequestID,
      query: query,
      status: "Looking through the workspace for \"\(query)\"..."
    )
  }

  func runtimeRequestIDForActiveSearch() -> String? {
    activeRuntimeRequestID
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
      ? "Search the open workspace for useful context."
      : "Open a workspace before searching."
  }

  func changedQueryStatus() -> String {
    clearActiveSearch()
    return "Query changed. Press Return to search again."
  }

  private func cancelActiveSearch() {
    taskSlot.cancel()
    activeQuery = nil
    activeRuntimeRequestID = nil
  }

  private func clearActiveSearch() {
    activeQuery = nil
    activeRuntimeRequestID = nil
    taskSlot.clear()
  }

  static func successStatus(query: String, matchCount: Int) -> String {
    if matchCount == 0 {
      return "No results for \"\(query)\"."
    }
    if matchCount == 1 {
      return "Found 1 useful match for \"\(query)\"."
    }
    return "Found \(matchCount) useful matches for \"\(query)\"."
  }

  static func failureStatus(error: Error) -> String {
    "Workspace search needs attention: \(error.localizedDescription)"
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
      return "Launch the local engine to search workspace files."
    }
    if !hasWorkspace {
      return "Open a workspace to search local files."
    }
    if query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
      return "Search file contents, symbols, or notes, then press Return."
    }
    if status.hasPrefix("No results") {
      return "No results yet. Try a shorter query, filename, or symbol name."
    }
    if status.hasPrefix("Workspace search needs attention") {
      return "Search failed. Check the local engine status, then try again."
    }

    return nil
  }

  static func overflowSummary(resultCount: Int) -> String? {
    guard resultCount > 8 else {
      return nil
    }

    return "Showing the first 8 matches. Narrow the query to focus the context."
  }
}
