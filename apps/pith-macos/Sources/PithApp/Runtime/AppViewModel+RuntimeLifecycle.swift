import Foundation

@MainActor
extension AppViewModel {
  func launchRuntime(launchDetail: String = "Launching local runtime") {
    guard runtimeState != .launching else {
      return
    }

    if runtimeState == .ready {
      runtimeBridge.stopRuntime(detail: "Relaunching local runtime...")
    }

    runtimeState = .launching
    runtimeDetail = launchDetail
    updateRuntimeConnectionState { state in
      state.clearLastFailureDetail()
    }

    Task {
      do {
        let bootstrap = try await RuntimeLaunchBootstrapLoader.load(
          runtimeBridge: runtimeBridge,
          launchDetail: launchDetail,
          lastWorkspacePath: AppPreferences.storedLastWorkspacePath(),
          isRestorablePath: isRestorableWorkspacePath,
          clearStoredWorkspace: AppPreferences.clearLastWorkspacePath
        )
        try await applyRuntimeLaunchBootstrap(bootstrap)
        announceFirstRequestReadyIfNeeded()
      } catch {
        applyRuntimeLaunchFailure(error)
      }
    }
  }

  func refreshModelHealthState(serverLabel: String? = nil) async {
    let modelRefresh = await RuntimeStateLoader.refreshModelHealth(
      using: runtimeBridge,
      serverLabel: serverLabel
    )
    updateLocalModelReadinessState { state in
      state.modelHealth = modelRefresh.modelHealth
    }
    if let runtimeDetail = modelRefresh.runtimeDetail {
      self.runtimeDetail = runtimeDetail
    }
    refreshLocalModelCatalog()
    await refreshRuntimeReadiness()
    announceFirstRequestReadyIfNeeded()
  }

  func refreshRuntimeReadiness() async {
    let readiness = await RuntimeStateLoader.refreshRuntimeReadiness(using: runtimeBridge)
    updateLocalModelReadinessState { state in
      state.runtimeReadiness = readiness
    }
  }

  func handleRuntimeConnectionStateChange(_ state: RuntimeBridge.ConnectionState, detail: String) {
    let plan = RuntimeConnectionStateReducer.plan(
      RuntimeConnectionStateSnapshot(
        previousState: runtimeState,
        nextState: state,
        detail: detail,
        lastFailureDetail: runtimeConnectionState.lastFailureDetail
      )
    )
    updateRuntimeConnectionState { runtimeConnectionState in
      runtimeConnectionState.applyConnectionUpdate(
        state: state,
        detail: detail,
        plan: plan
      )
    }

    if plan.clearsActiveTurnState {
      updateTimelineState { state in
        state.activeTurnID = nil
        state.activeTurnThreadID = nil
      }
      localExecutionRequests.clearAll()
    }

    if plan.clearsModelReadinessState {
      updateLocalModelReadinessState { state in
        state.clearRuntimeReadiness()
      }
    }

    if plan.shouldAppendFailureNotice {
      appendEntry(
        to: selectedThreadID,
        TimelineEventPresenter.runtimeDisconnected(detail: detail)
      )
    }
  }

  private func applyRuntimeLaunchBootstrap(_ bootstrap: RuntimeLaunchBootstrap) async throws {
    let currentWorkspace = bootstrap.workspaceRestore.workspace

    runtimeState = .ready
    await refreshModelHealthState(
      serverLabel: "\(bootstrap.session.serverName) \(bootstrap.session.serverVersion)"
    )
    applyMemoryStateRefresh(bootstrap.memoryRefresh, clearsMissing: true)
    await refreshPluginState()

    if let currentWorkspace {
      workspace = WorkspaceSummary(
        rootPath: currentWorkspace.rootPath,
        displayName: currentWorkspace.displayName
      )
      resetWorkspaceSearch()
      AppPreferences.storeLastWorkspacePath(currentWorkspace.rootPath)
    }

    if workspace != nil {
      try await refreshWorkspaceThreadSelection(
        from: bootstrap.threadList,
        createIfEmpty: isLocalModelReady()
      )
    } else {
      resetToWelcomeThread()
    }
    await refreshRuntimeReadiness()
    appendRuntimeLaunchAnnotations(bootstrap, currentWorkspace: currentWorkspace)
  }

  private func appendRuntimeLaunchAnnotations(
    _ bootstrap: RuntimeLaunchBootstrap,
    currentWorkspace: RuntimeBridge.RuntimeWorkspace?
  ) {
    let restoredWorkspaceSummary = bootstrap.workspaceRestore.restoredWorkspace
      ? currentWorkspace.map {
        WorkspaceSummary(rootPath: $0.rootPath, displayName: $0.displayName)
      }
      : nil

    RuntimeLaunchAnnotationFactory.entries(
      RuntimeLaunchAnnotationSnapshot(
        serverName: bootstrap.session.serverName,
        serverVersion: bootstrap.session.serverVersion,
        shouldAnnotateSetupLaunch: shouldAnnotateLaunchWithSetupEvents(),
        restoredWorkspace: restoredWorkspaceSummary,
        skippedWorkspaceRestorePath: bootstrap.workspaceRestore.skippedWorkspaceRestorePath,
        workspaceRestoreErrorDetail: bootstrap.workspaceRestore.restoreErrorDetail,
        modelHealth: modelHealth,
        isLocalModelReady: isLocalModelReady(),
        localModelRequiredSummary: localModelRequiredTimelineSummary()
      )
    ).forEach { entry in
      appendEntry(to: selectedThreadID, entry)
    }
  }

  private func applyRuntimeLaunchFailure(_ error: Error) {
    runtimeState = .failed
    runtimeDetail = error.localizedDescription
    updateLocalModelReadinessState { state in
      state.clearRuntimeReadiness()
    }
    updateMemoryState { state in
      state.resetRuntimeData()
    }
    updatePluginState { state in
      state.reset()
    }
    appendEntry(
      to: selectedThreadID,
      TimelineEventPresenter.runtimeLaunchFailed(error: error)
    )
  }

  private func isRestorableWorkspacePath(_ path: String) -> Bool {
    var isDirectory = ObjCBool(false)
    return FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory)
      && isDirectory.boolValue
  }
}
