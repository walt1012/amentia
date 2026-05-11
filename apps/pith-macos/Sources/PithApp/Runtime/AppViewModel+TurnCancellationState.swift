import Foundation

@MainActor
extension AppViewModel {
  func applyTurnCancellation(
    _ result: RuntimeBridge.RuntimeCancellationResult,
    previewThreadID: String
  ) async {
    appendItemsToTimeline(threadID: result.threadID, items: result.items)
    updateActiveTurn(threadID: result.threadID, activeTurnID: result.activeTurnID)
    refreshThreadPreview(
      threadID: previewThreadID,
      preview: TimelineEventPresenter.cancelledResponsePreview
    )
    await loadThreadHistory(threadID: result.threadID)
  }

  func requestPendingTurnCancellation() -> String? {
    guard let threadID = localExecutionRequests.pendingAgentThreadID() else {
      return nil
    }

    runtimeDetail = TimelineEventPresenter.cancellingTurnDetail
    refreshThreadPreview(
      threadID: threadID,
      preview: TimelineEventPresenter.cancellingResponsePreview
    )
    return threadID
  }

  func requestPendingApprovalCancellation() -> String? {
    guard let threadID = localExecutionRequests.pendingApprovalThreadID() else {
      return nil
    }

    runtimeDetail = TimelineEventPresenter.cancellingTurnDetail
    refreshThreadPreview(
      threadID: threadID,
      preview: TimelineEventPresenter.cancellingResponsePreview
    )
    return threadID
  }
}
