import Foundation

@MainActor
extension AppViewModel {
  func openWorkspace() {
    guard canOpenWorkspace() else {
      return
    }

    guard let url = AppFilePicker.chooseWorkspace() else {
      return
    }

    guard let requestToken = workspaceOpenCoordinator.begin(
      previousRuntimeDetail: runtimeDetail
    ) else {
      return
    }
    runtimeDetail = "Opening workspace..."

    Task {
      do {
        let bootstrap = try await WorkspaceOpenBootstrapLoader.load(
          runtimeBridge: runtimeBridge,
          path: url.path
        )
        guard workspaceOpenCoordinator.isCurrent(requestToken) else {
          return
        }
        try await applyWorkspaceOpenBootstrap(bootstrap)
        guard workspaceOpenCoordinator.isCurrent(requestToken) else {
          return
        }
        restoreRuntimeDetailAfterWorkspaceOpen(requestToken)
        workspaceOpenCoordinator.finish(requestToken)
        announceFirstRequestReadyIfNeeded()
      } catch {
        guard workspaceOpenCoordinator.isCurrent(requestToken) else {
          return
        }
        restoreRuntimeDetailAfterWorkspaceOpen(requestToken)
        workspaceOpenCoordinator.finish(requestToken)
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.workspaceOpenFailed(error: error)
        )
      }
    }
  }

  private func applyWorkspaceOpenBootstrap(_ bootstrap: WorkspaceOpenBootstrap) async throws {
    workspace = WorkspaceSummary(
      rootPath: bootstrap.workspace.rootPath,
      displayName: bootstrap.workspace.displayName
    )
    resetWorkspaceSearch()
    AppPreferences.storeLastWorkspacePath(bootstrap.workspace.rootPath)
    applyMemoryStateRefresh(bootstrap.memoryRefresh, clearsMissing: false)
    try await refreshWorkspaceThreadSelection(
      from: bootstrap.threadList,
      createIfEmpty: isLocalModelReady()
    )
    await refreshRuntimeReadiness()
    appendWorkspaceOpenedEvent(bootstrap.workspace)
  }

  private func appendWorkspaceOpenedEvent(_ workspace: RuntimeBridge.RuntimeWorkspace) {
    appendEntry(
      to: selectedThreadID,
      TimelineEventPresenter.workspaceOpened(workspace)
    )
  }

  private func restoreRuntimeDetailAfterWorkspaceOpen(_ token: WorkspaceOpenRequestToken) {
    guard runtimeState == .ready else {
      return
    }

    runtimeDetail = modelDownloadCoordinator.isDownloading
      ? modelDownloadProgressSummary()
      : token.previousRuntimeDetail
  }
}
