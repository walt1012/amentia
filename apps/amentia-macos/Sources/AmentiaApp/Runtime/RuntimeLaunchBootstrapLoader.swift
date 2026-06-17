import Foundation

struct RuntimeLaunchBootstrap {
  let session: RuntimeBridge.SessionInfo
  let memoryRefresh: MemoryStateRefresh
  let workspaceRestore: RuntimeWorkspaceRestoreResult
  let threadList: [RuntimeBridge.RuntimeThreadSummary]
}

@MainActor
enum RuntimeLaunchBootstrapLoader {
  static func load(
    runtimeBridge: RuntimeBridge,
    launchDetail: String,
    lastWorkspacePath: String?,
    isRestorablePath: (String) -> Bool,
    clearStoredWorkspace: () -> Void
  ) async throws -> RuntimeLaunchBootstrap {
    let session = try await runtimeBridge.launchAndInitialize(launchDetail: launchDetail)
    let memoryRefresh = await MemoryStateLoader.refresh(using: runtimeBridge)
    let currentWorkspace = try? await runtimeBridge.currentWorkspace()
    let workspaceRestore = await RuntimeWorkspaceRestorer.restore(
      currentWorkspace: currentWorkspace,
      lastWorkspacePath: lastWorkspacePath,
      isRestorablePath: isRestorablePath,
      openWorkspace: { [runtimeBridge] path in
        try await runtimeBridge.openWorkspace(path: path)
      },
      clearStoredWorkspace: clearStoredWorkspace
    )
    let threadList = try await runtimeBridge.listThreads()

    return RuntimeLaunchBootstrap(
      session: session,
      memoryRefresh: memoryRefresh,
      workspaceRestore: workspaceRestore,
      threadList: threadList
    )
  }
}
