import Foundation

@MainActor
extension AppViewModel {
  func canSearchWorkspace() -> Bool {
    let query = WorkspaceSearchSession.trimmedQuery(workspaceSearchQuery)
    return WorkspaceSearchSession.canSearch(workspaceSearchSnapshot())
      && workspaceSearchSession.canStart(query: query)
  }

  func searchWorkspace() {
    guard canSearchWorkspace() else {
      return
    }

    let query = WorkspaceSearchSession.trimmedQuery(workspaceSearchQuery)
    let shouldCancelRunningSearch = isWorkspaceSearching
    let requestToken = workspaceSearchSession.begin(query: query)

    updateWorkspaceSearchState { state in
      state.begin(requestToken)
    }
    let task = Task {
      defer {
        workspaceSearchSession.finish(requestToken)
      }
      do {
        if shouldCancelRunningSearch {
          _ = try? await runtimeBridge.cancelWorkspaceSearches()
          guard workspaceSearchSession.isCurrent(requestToken) else {
            return
          }
        }
        let matches = try await runtimeBridge.searchWorkspace(query: requestToken.query)
        guard workspaceSearchSession.isCurrent(requestToken) else {
          return
        }
        guard !WorkspaceSearchSession.queryChanged(
          currentQuery: workspaceSearchQuery,
          token: requestToken
        ) else {
          finishChangedWorkspaceSearch()
          return
        }
        updateWorkspaceSearchState { state in
          state.finishWithMatches(
            WorkspaceSearchSession.matchSummaries(from: matches),
            status: WorkspaceSearchSession.successStatus(
              query: requestToken.query,
              matchCount: matches.count
            )
          )
        }
      } catch {
        guard workspaceSearchSession.isCurrent(requestToken) else {
          return
        }
        guard !WorkspaceSearchSession.queryChanged(
          currentQuery: workspaceSearchQuery,
          token: requestToken
        ) else {
          finishChangedWorkspaceSearch()
          return
        }
        updateWorkspaceSearchState { state in
          state.finishWithFailure(
            status: WorkspaceSearchSession.failureStatus(error: error)
          )
        }
      }
    }
    workspaceSearchSession.bind(task: task, token: requestToken)
  }

  func clearWorkspaceSearch() {
    updateWorkspaceSearchState { state in
      state.query = ""
    }
    resetWorkspaceSearch()
  }

  func workspaceSearchEmptyStateSummary() -> String? {
    WorkspaceSearchSession.emptyStateSummary(
      runtimeState: runtimeState,
      hasWorkspace: workspace != nil,
      query: workspaceSearchQuery,
      status: workspaceSearchStatus,
      isSearching: isWorkspaceSearching,
      hasResults: !workspaceSearchResults.isEmpty
    )
  }

  func workspaceSearchOverflowSummary() -> String? {
    WorkspaceSearchSession.overflowSummary(resultCount: workspaceSearchResults.count)
  }

  private func workspaceSearchSnapshot() -> WorkspaceSearchSnapshot {
    WorkspaceSearchSnapshot(
      runtimeState: runtimeState,
      hasWorkspace: workspace != nil,
      isSearching: isWorkspaceSearching,
      query: workspaceSearchQuery
    )
  }

  func resetWorkspaceSearch() {
    cancelRuntimeWorkspaceSearches()
    let resetStatus = workspaceSearchSession.resetStatus(hasWorkspace: workspace != nil)
    updateWorkspaceSearchState { state in
      state.reset(status: resetStatus)
    }
  }

  private func finishChangedWorkspaceSearch() {
    let changedStatus = workspaceSearchSession.changedQueryStatus()
    updateWorkspaceSearchState { state in
      state.finishWithChangedQuery(status: changedStatus)
    }
  }

  private func cancelRuntimeWorkspaceSearches() {
    guard isWorkspaceSearching else {
      return
    }

    Task {
      _ = try? await runtimeBridge.cancelWorkspaceSearches()
    }
  }
}
