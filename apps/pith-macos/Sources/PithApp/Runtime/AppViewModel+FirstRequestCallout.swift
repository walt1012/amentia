import Foundation

@MainActor
extension AppViewModel {
  func shouldShowFirstRequestCallout() -> Bool {
    canUseComposer()
      && trimmedDraftMessage.isEmpty
      && selectedThreadIsWaitingForFirstMessage()
  }

  func firstRequestCalloutTitle() -> String {
    "Start Cowork Session"
  }

  func firstRequestCalloutSummary() -> String {
    FirstRequestPromptPresenter.calloutSummary()
  }

  func firstRequestCalloutDetail() -> String {
    FirstRequestPromptPresenter.calloutDetail(workspaceDisplayName: workspace?.displayName)
  }

  func firstRequestCalloutActionTitle() -> String? {
    FirstRequestPromptPresenter.primaryActionTitle(
      for: firstRequestSuggestion(id: FirstRequestPromptPresenter.mapWorkspaceID)
    )
  }

  func canRunFirstRequestCalloutAction() -> Bool {
    firstRequestSuggestion(id: FirstRequestPromptPresenter.mapWorkspaceID) != nil
  }

  func runFirstRequestCalloutAction() {
    guard canRunFirstRequestCalloutAction() else {
      return
    }

    useFirstRequestSuggestion(id: FirstRequestPromptPresenter.mapWorkspaceID)
  }

  func firstRequestCalloutSecondaryActionTitle() -> String? {
    FirstRequestPromptPresenter.secondaryActionTitle(
      for: firstRequestSuggestion(id: FirstRequestPromptPresenter.planNextStepID)
    )
  }

  func canRunFirstRequestCalloutSecondaryAction() -> Bool {
    firstRequestSuggestion(id: FirstRequestPromptPresenter.planNextStepID) != nil
  }

  func runFirstRequestCalloutSecondaryAction() {
    guard canRunFirstRequestCalloutSecondaryAction() else {
      return
    }

    useFirstRequestSuggestion(id: FirstRequestPromptPresenter.planNextStepID)
  }

  func firstRequestSuggestion(id: String) -> ComposerSuggestionSummary? {
    guard canUseComposer(),
          trimmedDraftMessage.isEmpty,
          selectedThreadIsWaitingForFirstMessage()
    else {
      return nil
    }

    return FirstRequestPromptPresenter.suggestion(id: id, workspaceDisplayName: workspace?.displayName)
  }

  func useFirstRequestSuggestion(id: String) {
    guard let suggestion = firstRequestSuggestion(id: id) else {
      return
    }

    draftMessage = suggestion.message
  }

  func selectedThreadIsWaitingForFirstMessage() -> Bool {
    timelineState.isWaitingForFirstMessage()
  }

  func shouldAnnotateLaunchWithSetupEvents() -> Bool {
    SetupFlowState.shouldAnnotateLaunch(setupFlowSnapshot())
  }

  func localModelRequiredTimelineSummary() -> String {
    localModelSetupGuidance().summary
  }

  func announceFirstRequestReadyIfNeeded() {
    guard SetupFlowState.isCoreReadyForFirstRequest(setupFlowSnapshot()),
          let threadID = selectedThreadID,
          !threadID.hasPrefix("local-"),
          selectedThreadIsWaitingForFirstMessage(),
          !timelineState.hasAnnouncedSetupComplete(for: threadID)
    else {
      return
    }

    updateTimelineState { state in
      state.markSetupCompleteAnnounced(threadID: threadID)
    }
    appendEntry(
      to: threadID,
      TimelineEventPresenter.firstRequestReady()
    )
  }
}
