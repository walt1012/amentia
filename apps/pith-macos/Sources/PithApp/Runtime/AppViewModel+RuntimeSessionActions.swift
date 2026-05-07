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
    case .useFirstRequestPrompt:
      useFirstRequestSuggestion(id: FirstRequestPromptPresenter.mapWorkspaceID)
    case .sendFirstRequest:
      sendDraftMessage()
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
    let snapshot = sessionActionSnapshot()
    return SessionActionPlanner.runtimePrimaryActionTitle(
      for: SessionActionPlanner.runtimePrimaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func canRunRuntimePrimaryAction() -> Bool {
    let snapshot = sessionActionSnapshot()
    return SessionActionPlanner.canRunRuntimePrimaryAction(
      SessionActionPlanner.runtimePrimaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func runRuntimePrimaryAction() {
    let snapshot = sessionActionSnapshot()
    guard let action = SessionActionPlanner.runtimePrimaryAction(snapshot),
          SessionActionPlanner.canRunRuntimePrimaryAction(action, snapshot: snapshot)
    else {
      return
    }

    switch action {
    case .launchRuntime:
      launchRuntime()
    case .cancelTurn:
      cancelActiveTurn()
    }
  }

  func canLaunchRuntime() -> Bool {
    SessionActionPlanner.canLaunchRuntime(sessionActionSnapshot())
  }

  func canOpenWorkspace() -> Bool {
    SessionActionPlanner.canOpenWorkspace(sessionActionSnapshot())
  }

  func canCreateThread() -> Bool {
    SessionActionPlanner.canCreateThread(sessionActionSnapshot())
  }

  func canInstallPlugin() -> Bool {
    SessionActionPlanner.canInstallPlugin(sessionActionSnapshot())
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
}
