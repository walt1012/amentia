import Foundation

enum WorkspaceThreadSelectionLoader {
  static func load(
    workspace: WorkspaceSummary,
    runtimeThreads: [RuntimeBridge.RuntimeThreadSummary],
    createIfEmpty: Bool,
    startThread: (String) async throws -> ThreadSummary
  ) async throws -> [ThreadSummary] {
    var workspaceThreads = runtimeThreads
      .filter { $0.workspaceRootPath == workspace.rootPath }
      .map { RuntimeSummaryMapper.threadSummary(from: $0) }

    if workspaceThreads.isEmpty && createIfEmpty {
      let thread = try await startThread("\(workspace.displayName) Thread")
      workspaceThreads = [thread]
    }

    return workspaceThreads
  }
}
