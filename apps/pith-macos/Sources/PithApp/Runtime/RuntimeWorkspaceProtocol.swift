import Foundation

struct RuntimeWorkspacePayload: Codable {
  let rootPath: String
  let displayName: String
}

struct WorkspaceOpenParams: Codable {
  let path: String
}

struct WorkspaceOpenResult: Codable {
  let workspace: RuntimeWorkspacePayload
  let threadCount: Int
}

struct WorkspaceCurrentResult: Codable {
  let workspace: RuntimeWorkspacePayload?
}

struct WorkspaceSearchParams: Codable {
  let query: String
  let maxResults: Int
}

struct WorkspaceSearchResult: Codable {
  let query: String
  let workspace: RuntimeWorkspacePayload
  let matches: [WorkspaceSearchMatchPayload]
}

struct WorkspaceSearchMatchPayload: Codable {
  let relativePath: String
  let lineNumber: Int
  let line: String
}

extension RuntimeBridge {
  struct RuntimeWorkspace {
    let rootPath: String
    let displayName: String
    let threadCount: Int
  }

  struct RuntimeWorkspaceSearchMatch {
    let relativePath: String
    let lineNumber: Int
    let line: String
  }
}
