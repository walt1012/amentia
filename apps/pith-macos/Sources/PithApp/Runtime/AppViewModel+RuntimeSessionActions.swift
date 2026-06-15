import Foundation

@MainActor
extension AppViewModel {
  func readinessStepActionTitle(_ step: ReadinessStepSummary) -> String? {
    let snapshot = runtimeReadinessActionSnapshot()
    return RuntimeReadinessActionPlanner.title(
      for: RuntimeReadinessActionPlanner.action(for: step, snapshot: snapshot),
      snapshot: snapshot
    )
  }

  func canRunReadinessStepAction(_ step: ReadinessStepSummary) -> Bool {
    let snapshot = runtimeReadinessActionSnapshot()
    return RuntimeReadinessActionPlanner.canRun(
      RuntimeReadinessActionPlanner.action(for: step, snapshot: snapshot),
      snapshot: snapshot
    )
  }

  func runReadinessStepAction(_ step: ReadinessStepSummary) {
    let snapshot = runtimeReadinessActionSnapshot()
    guard let action = RuntimeReadinessActionPlanner.action(for: step, snapshot: snapshot),
          RuntimeReadinessActionPlanner.canRun(action, snapshot: snapshot)
    else {
      return
    }

    switch action {
    case .launchRuntime:
      launchRuntime()
    case .setupModel:
      runModelSetupCalloutAction()
    case .openWorkspace:
      openWorkspace()
    case .createThread:
      createThread()
    case .sendFirstRequest:
      sendDraftMessage()
    case .enableWebSearchPlugin:
      setPluginEnabled(pluginID: webSearchPluginID, enabled: true)
    case .openPluginAccess:
      pluginManagerSection = .access
      runtimeDetail = "Open Advanced > Plugins to enable Web Search access."
    case .openPluginCommands:
      pluginManagerSection = .commands
      runtimeDetail = "Open Advanced > Plugins to enable a ready action."
    case .inspectSandboxStatus:
      runtimeDetail = "Native sandbox status is shown with Pith status and action receipts."
    }
  }

  func setupCalloutActionTitle() -> String? {
    SetupCalloutPresenter.primaryActionTitle(setupCalloutSnapshot())
  }

  func canRunSetupCalloutAction() -> Bool {
    let snapshot = setupCalloutActionSnapshot()
    return SetupCalloutActionPlanner.canRun(
      SetupCalloutActionPlanner.primaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func runSetupCalloutAction() {
    let snapshot = setupCalloutActionSnapshot()
    guard let action = SetupCalloutActionPlanner.primaryAction(snapshot),
          SetupCalloutActionPlanner.canRun(action, snapshot: snapshot)
    else {
      return
    }

    switch action {
    case .launchRuntime:
      launchRuntime()
    case .setupModel:
      runModelSetupCalloutAction()
    case .openWorkspace:
      openWorkspace()
    case .createThread:
      createThread()
    }
  }

  func setupCalloutSecondaryActionTitle() -> String? {
    SetupCalloutPresenter.secondaryActionTitle(setupCalloutSnapshot())
  }

  func canRunSetupCalloutSecondaryAction() -> Bool {
    let snapshot = setupCalloutActionSnapshot()
    return SetupCalloutActionPlanner.canRun(
      SetupCalloutActionPlanner.secondaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func runSetupCalloutSecondaryAction() {
    let snapshot = setupCalloutActionSnapshot()
    guard let action = SetupCalloutActionPlanner.secondaryAction(snapshot),
          SetupCalloutActionPlanner.canRun(action, snapshot: snapshot)
    else {
      return
    }

    switch action {
    case .setupModelSecondary:
      runModelSetupCalloutSecondaryAction()
    }
  }

  func runtimePrimaryActionTitle() -> String? {
    SessionActionPlanner.runtimePrimaryActionTitle(sessionActionSnapshot())
  }

  func canRunRuntimePrimaryAction() -> Bool {
    SessionActionPlanner.canRunRuntimePrimaryAction(sessionActionSnapshot())
  }

  func runRuntimePrimaryAction() {
    guard canRunRuntimePrimaryAction() else {
      return
    }

    cancelActiveTurn()
  }

  func canLaunchRuntime() -> Bool {
    SessionActionPlanner.canLaunchRuntime(sessionActionSnapshot())
  }

  func canOpenWorkspace() -> Bool {
    SessionActionPlanner.canOpenWorkspace(sessionActionSnapshot())
      && !workspaceOpenCoordinator.isOpening
  }

  func canCreateThread() -> Bool {
    SessionActionPlanner.canCreateThread(sessionActionSnapshot())
      && !threadCreationCoordinator.isCreating
  }

  func canInstallPlugin() -> Bool {
    SessionActionPlanner.canInstallPlugin(sessionActionSnapshot())
      && !hasPluginLifecycleOperation()
  }

  func canSendDraftMessage() -> Bool {
    SessionActionPlanner.canSendDraftMessage(sessionActionSnapshot())
  }

  func canCancelActiveTurn() -> Bool {
    SessionActionPlanner.canCancelActiveTurn(sessionActionSnapshot())
  }

  func canRespondToApproval(approvalID: String) -> Bool {
    SessionActionPlanner.canRespondToApproval(
      approvalID: approvalID,
      snapshot: sessionActionSnapshot()
    )
  }

  func canUseComposer() -> Bool {
    SessionActionPlanner.canUseComposer(sessionActionSnapshot())
  }

  func canEnableWebSearchPlugin() -> Bool {
    guard let plugin = pluginSummary(pluginID: webSearchPluginID),
          !plugin.enabled
    else {
      return false
    }

    return canSetPluginEnabled(pluginID: webSearchPluginID)
  }
}

private let webSearchPluginID = "web-search"
