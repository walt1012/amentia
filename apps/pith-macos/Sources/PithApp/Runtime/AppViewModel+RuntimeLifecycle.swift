import Foundation

@MainActor
extension AppViewModel {
  func startDailyUseSessionIfNeeded() {
    guard runtimeState == .disconnected else {
      return
    }

    launchRuntime(launchDetail: "Preparing local engine for daily use")
  }

  func launchRuntime(launchDetail: String = "Launching local engine") {
    guard runtimeState != .launching else {
      return
    }

    if runtimeState == .ready {
      runtimeBridge.stopRuntime(detail: "Relaunching local engine...")
    }

    runtimeState = .launching
    runtimeDetail = launchDetail
    updateRuntimeConnectionState { state in
      state.clearLastFailureDetail()
    }
    let launchToken = runtimeLaunchCoordinator.begin()
    let failureThreadID = selectedThreadID

    let task = Task {
      do {
        let bootstrap = try await RuntimeLaunchBootstrapLoader.load(
          runtimeBridge: runtimeBridge,
          launchDetail: launchDetail,
          lastWorkspacePath: AppPreferences.storedLastWorkspacePath(),
          isRestorablePath: isRestorableWorkspacePath,
          clearStoredWorkspace: AppPreferences.clearLastWorkspacePath
        )
        guard runtimeLaunchCoordinator.isCurrent(launchToken) else {
          return
        }
        try await applyRuntimeLaunchBootstrap(bootstrap)
        guard runtimeLaunchCoordinator.isCurrent(launchToken) else {
          return
        }
        runtimeLaunchCoordinator.finish(launchToken)
        announceFirstRequestReadyIfNeeded()
      } catch {
        guard runtimeLaunchCoordinator.isCurrent(launchToken) else {
          return
        }
        runtimeLaunchCoordinator.finish(launchToken)
        applyRuntimeLaunchFailure(error, timelineThreadID: failureThreadID)
      }
    }
    runtimeLaunchCoordinator.bind(task: task, token: launchToken)
  }

  func refreshModelHealthState(
    serverLabel: String? = nil,
    announcesFirstRequestReady: Bool = true
  ) async {
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
    if announcesFirstRequestReady {
      announceFirstRequestReadyIfNeeded()
    }
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
      turnCancellationCoordinator.cancel()
      runtimeLaunchCoordinator.cancel()
      workspaceOpenCoordinator.cancel()
      threadCreationCoordinator.cancel()
      threadHistoryLoadCoordinator.cancel()
      localModelMetadataCoordinator.cancel()
      pluginLifecycleOperations.cancel()
      updatePluginState { state in
        state.resetLifecycleOperation()
      }
      resetWorkspaceSearch()
    }

    if plan.clearsModelReadinessState {
      updateLocalModelReadinessState { state in
        state.clearRuntimeReadiness()
      }
    }

    if plan.clearsRuntimeDerivedState {
      updateMemoryState { state in
        state.resetRuntimeData()
      }
      updatePluginState { state in
        state.reset()
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
      serverLabel: "\(bootstrap.session.serverName) \(bootstrap.session.serverVersion)",
      announcesFirstRequestReady: false
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

  private func applyRuntimeLaunchFailure(_ error: Error, timelineThreadID: ThreadSummary.ID?) {
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
      to: timelineThreadID,
      TimelineEventPresenter.runtimeLaunchFailed(error: error)
    )
  }

  private func isRestorableWorkspacePath(_ path: String) -> Bool {
    var isDirectory = ObjCBool(false)
    return FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory)
      && isDirectory.boolValue
  }
}

struct RuntimeLaunchRequestToken: Equatable {
  fileprivate let id: UUID
}

final class RuntimeLaunchCoordinator {
  private let taskSlot = CancellableTaskSlot()

  func begin() -> RuntimeLaunchRequestToken {
    RuntimeLaunchRequestToken(id: taskSlot.replace())
  }

  func bind(task: Task<Void, Never>, token: RuntimeLaunchRequestToken) {
    taskSlot.bind(task: task, requestID: token.id)
  }

  func isCurrent(_ token: RuntimeLaunchRequestToken) -> Bool {
    taskSlot.isCurrent(token.id)
  }

  func finish(_ token: RuntimeLaunchRequestToken) {
    taskSlot.finish(token.id)
  }

  func cancel() {
    taskSlot.cancel()
  }
}
