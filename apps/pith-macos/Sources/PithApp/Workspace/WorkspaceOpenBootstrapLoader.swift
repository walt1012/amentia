import Foundation

struct WorkspaceOpenBootstrap {
  let workspace: RuntimeBridge.RuntimeWorkspace
  let memoryRefresh: MemoryStateRefresh
  let threadList: [RuntimeBridge.RuntimeThreadSummary]
}

enum WorkspaceOpenBootstrapLoader {
  static func load(
    runtimeBridge: RuntimeBridge,
    path: String
  ) async throws -> WorkspaceOpenBootstrap {
    let workspace = try await runtimeBridge.openWorkspace(path: path)
    let memoryRefresh = await MemoryStateLoader.refresh(using: runtimeBridge)
    let threadList = try await runtimeBridge.listThreads()

    return WorkspaceOpenBootstrap(
      workspace: workspace,
      memoryRefresh: memoryRefresh,
      threadList: threadList
    )
  }
}
