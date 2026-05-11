import Foundation

@MainActor
extension AppViewModel {
  func runtimeHeaderSnapshot() -> RuntimeHeaderSnapshot {
    let isModelReady = isLocalModelReady()
    let modelSetupSummary = runtimeState == .ready && !isModelReady
      ? localModelSetupGuidance().summary
      : ""
    return RuntimeHeaderSnapshot(
      runtimeState: runtimeState,
      runtimeDetail: runtimeDetail,
      modelSetupSummary: modelSetupSummary,
      isLocalModelReady: isModelReady,
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      hasActiveTurn: hasActiveOrPendingTurn(),
      isWaitingForFirstMessage: selectedThreadIsWaitingForFirstMessage(),
      hasDraftMessage: !trimmedDraftMessage.isEmpty,
      isWorkspaceSearching: isWorkspaceSearching,
      hasModelDownload: modelDownloadState.hasActiveDownload,
      hasPausedModelDownload: modelDownloadState.hasPausedDownload
    )
  }

  func setupProgressSnapshot() -> SetupProgressSnapshot {
    let isModelReady = isLocalModelReady()
    let modelReadinessDetail = runtimeState == .ready && !isModelReady
      ? localModelSetupGuidance().readinessDetail
      : ""
    return SetupProgressSnapshot(
      readyStepCount: setupReadyStepCount(),
      stepCount: SetupFlowState.stepCount,
      runtimeState: runtimeState,
      showsRuntimeActivity: showsRuntimeActivity(),
      isLocalModelReady: isModelReady,
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      hasActiveTurn: hasActiveOrPendingTurn(),
      isWaitingForFirstMessage: selectedThreadIsWaitingForFirstMessage(),
      hasDraft: !trimmedDraftMessage.isEmpty,
      modelReadinessDetail: modelReadinessDetail
    )
  }

  func runtimeReadinessSnapshot() -> RuntimeReadinessSnapshot {
    let isModelReady = isLocalModelReady()
    let modelGuidance = runtimeState == .ready
      ? localModelSetupGuidance()
      : nil
    return RuntimeReadinessSnapshot(
      runtimeState: runtimeState,
      modelReadinessDetail: modelGuidance?.readinessDetail ?? "Waiting",
      modelTone: modelGuidance?.tone ?? .neutral,
      workspaceDisplayName: workspace?.displayName,
      isLocalModelReady: isModelReady,
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      hasActiveTurn: hasActiveOrPendingTurn(),
      isWaitingForFirstMessage: selectedThreadIsWaitingForFirstMessage(),
      hasDraftMessage: !trimmedDraftMessage.isEmpty
    )
  }

  func inspectorSessionSnapshot() -> InspectorSessionSnapshot {
    return InspectorSessionSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady(),
      hasWorkspace: workspace != nil,
      workspaceDisplayName: workspace?.displayName,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      selectedThreadTitle: selectedThreadTitle(),
      hasActiveTurn: hasActiveOrPendingTurn(),
      setupReadyStepCount: setupReadyStepCount(),
      setupStepCount: SetupFlowState.stepCount,
      setupProgressDetail: setupProgressDetail(),
      isWaitingForFirstMessage: selectedThreadIsWaitingForFirstMessage(),
      runtimeReadinessStatus: runtimeReadiness?.status,
      runtimeReadinessChecks: runtimeReadiness?.checks ?? []
    )
  }

  func setupCalloutSnapshot() -> SetupCalloutSnapshot {
    let modelProgressDetail: String?
    if shouldShowModelDownloadProgress() {
      modelProgressDetail = modelDownloadProgressSummary()
    } else {
      modelProgressDetail = nil
    }

    return SetupCalloutSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady(),
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      modelGuidance: localModelSetupGuidance(),
      modelProgressDetail: modelProgressDetail,
      runtimeLaunchActionTitle: runtimeLaunchButtonTitle(),
      modelPrimaryActionTitle: modelSetupCalloutActionTitle(),
      modelSecondaryActionTitle: modelSetupCalloutSecondaryActionTitle()
    )
  }

  func setupCalloutActionSnapshot() -> SetupCalloutActionSnapshot {
    SetupCalloutActionSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady(),
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      canLaunchRuntime: canLaunchRuntime(),
      canRunModelSetupAction: canRunModelSetupCalloutAction(),
      canRunModelSetupSecondaryAction: canRunModelSetupCalloutSecondaryAction(),
      canOpenWorkspace: canOpenWorkspace(),
      canCreateThread: canCreateThread()
    )
  }

  func setupFlowSnapshot() -> SetupFlowSnapshot {
    SetupFlowSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady(),
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      isWaitingForFirstMessage: selectedThreadIsWaitingForFirstMessage()
    )
  }

  func setupReadyStepCount() -> Int {
    SetupFlowState.readyStepCount(setupFlowSnapshot())
  }

  func hasRuntimeThreadSelection() -> Bool {
    timelineState.hasRuntimeThreadSelection(workspace: workspace)
  }

  func sessionActionSnapshot() -> SessionActionSnapshot {
    return SessionActionSnapshot(
      runtimeState: runtimeState,
      hasWorkspace: workspace != nil,
      isLocalModelReady: isLocalModelReady(),
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      hasActiveOrPendingTurn: hasActiveOrPendingTurn(),
      hasCancelableTurn: hasCancelableLocalExecution(),
      hasDraftMessage: !trimmedDraftMessage.isEmpty,
      pendingApprovalIDs: timelineState.selectedPendingApprovalIDs
    )
  }

  func runtimeReadinessActionSnapshot() -> RuntimeReadinessActionSnapshot {
    RuntimeReadinessActionSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady(),
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      canLaunchRuntime: canLaunchRuntime(),
      canRunModelSetupAction: canRunModelSetupCalloutAction(),
      canOpenWorkspace: canOpenWorkspace(),
      canCreateThread: canCreateThread(),
      canUseComposer: canUseComposer(),
      isWaitingForFirstMessage: selectedThreadIsWaitingForFirstMessage(),
      hasDraftMessage: !trimmedDraftMessage.isEmpty,
      hasFirstRequestSuggestion: firstRequestSuggestion(id: FirstRequestPromptPresenter.mapWorkspaceID) != nil,
      runtimeLaunchButtonTitle: runtimeLaunchButtonTitle(),
      modelSetupActionTitle: modelSetupCalloutActionTitle()
    )
  }
}
