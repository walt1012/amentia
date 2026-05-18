import Foundation

extension RuntimeBridge {
  func openWorkspace(path: String) async throws -> RuntimeWorkspace {
    let response: JSONRPCResponse<WorkspaceOpenResult> = try await sendRequest(
      method: "workspace/open",
      params: WorkspaceOpenParams(path: path)
    )
    let result = try responseResult(from: response)

    return RuntimeWorkspace(
      rootPath: result.workspace.rootPath,
      displayName: result.workspace.displayName,
      threadCount: result.threadCount
    )
  }

  func currentWorkspace() async throws -> RuntimeWorkspace? {
    let response: JSONRPCResponse<WorkspaceCurrentResult> = try await sendRequest(
      method: "workspace/current",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    guard let workspace = result.workspace else {
      return nil
    }

    return RuntimeWorkspace(
      rootPath: workspace.rootPath,
      displayName: workspace.displayName,
      threadCount: 0
    )
  }

  func searchWorkspace(query: String, maxResults: Int = 24) async throws -> [RuntimeWorkspaceSearchMatch] {
    let response: JSONRPCResponse<WorkspaceSearchResult> = try await sendRequest(
      method: "workspace/search",
      params: WorkspaceSearchParams(query: query, maxResults: maxResults)
    )
    let result = try responseResult(from: response)

    return result.matches.map { match in
      RuntimeWorkspaceSearchMatch(
        relativePath: match.relativePath,
        lineNumber: match.lineNumber,
        line: match.line
      )
    }
  }

  func cancelWorkspaceSearches() async throws -> Int {
    let response: JSONRPCResponse<WorkspaceSearchCancelRunningResult> = try await sendRequest(
      method: "workspace/searchCancelRunning",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return result.cancelledCount
  }
}
