import Foundation

struct RuntimeWorkspaceRestoreResult {
  let workspace: RuntimeBridge.RuntimeWorkspace?
  let restoredWorkspace: Bool
  let skippedWorkspaceRestorePath: String?
  let restoreErrorDetail: String?
}

@MainActor
enum RuntimeWorkspaceRestorer {
  static func restore(
    currentWorkspace: RuntimeBridge.RuntimeWorkspace?,
    lastWorkspacePath: String?,
    isRestorablePath: (String) -> Bool,
    openWorkspace: (String) async throws -> RuntimeBridge.RuntimeWorkspace,
    clearStoredWorkspace: () -> Void
  ) async -> RuntimeWorkspaceRestoreResult {
    var workspace = currentWorkspace
    var restoredWorkspace = false
    var skippedWorkspaceRestorePath: String?
    var restoreErrorDetail: String?

    if workspace == nil, let lastWorkspacePath {
      if isRestorablePath(lastWorkspacePath) {
        do {
          workspace = try await openWorkspace(lastWorkspacePath)
          restoredWorkspace = true
        } catch {
          restoreErrorDetail = error.localizedDescription
        }
      } else {
        skippedWorkspaceRestorePath = lastWorkspacePath
        clearStoredWorkspace()
      }
    }

    return RuntimeWorkspaceRestoreResult(
      workspace: workspace,
      restoredWorkspace: restoredWorkspace,
      skippedWorkspaceRestorePath: skippedWorkspaceRestorePath,
      restoreErrorDetail: restoreErrorDetail
    )
  }
}
