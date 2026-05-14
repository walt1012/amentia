import Foundation

@MainActor
extension AppViewModel {
  func applyRuntimeTurnResult(
    _ result: RuntimeBridge.RuntimeTurnResult,
    refreshMemory: Bool = false
  ) async {
    appendItemsToTimeline(threadID: result.threadID, items: result.items)
    updatePendingApprovals(threadID: result.threadID, approvals: result.pendingApprovals)
    updateActiveTurn(threadID: result.threadID, activeTurnID: result.activeTurnID)
    let wasCancelled = result.items.contains {
      $0.attributes["streamingStatus"] == "cancelled"
    }
    let pluginCommandCancelled = result.items.contains {
      $0.attributes["pluginCommandStatus"] == "cancelled"
    }
    let pluginCommandFailed = result.items.contains {
      $0.attributes["pluginCommandStatus"] == "failed"
    }
    if pluginCommandCancelled {
      runtimeDetail = TimelineEventPresenter.pendingPluginCommandCancelledDetail
    } else if wasCancelled {
      runtimeDetail = TimelineEventPresenter.pendingTurnCancelledDetail
    } else if pluginCommandFailed {
      runtimeDetail = TimelineEventPresenter.pluginCommandFailureDetail(from: result.items)
    }
    let preview: String
    if pluginCommandCancelled {
      preview = TimelineEventPresenter.cancelledPluginCommandPreview
    } else if wasCancelled {
      preview = TimelineEventPresenter.cancelledResponsePreview
    } else if pluginCommandFailed {
      preview = TimelineEventPresenter.failedPluginCommandPreview
    } else {
      preview = TimelineEventPresenter.turnPreview(
        turnID: result.turnID,
        activeTurnID: result.activeTurnID
      )
    }
    refreshThreadPreview(
      threadID: result.threadID,
      preview: preview
    )

    if refreshMemory {
      await refreshMemoryState()
    }
  }

  func beginPendingLocalTurn(threadID: String) -> UUID {
    draftMessage = ""
    runtimeDetail = TimelineEventPresenter.generatingLocalResponseDetail
    return localExecutionRequests.beginAgentRequest(threadID: threadID)
  }

  func applyPendingTurnCancellation(threadID: String) {
    runtimeDetail = TimelineEventPresenter.pendingTurnCancelledDetail
    refreshThreadPreview(
      threadID: threadID,
      preview: TimelineEventPresenter.cancelledResponsePreview
    )
    appendEntry(
      to: threadID,
      TimelineEventPresenter.pendingTurnCancelled()
    )
  }

  func applyPendingTurnFailure(
    threadID: String,
    message: String,
    error: Error
  ) {
    if draftMessage.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
      draftMessage = message
    }
    runtimeDetail = error.localizedDescription
    appendEntry(
      to: threadID,
      TimelineEventPresenter.turnFailed(error: error)
    )
  }

  func applyApprovalResponse(_ result: RuntimeBridge.RuntimeApprovalResponse) async {
    appendItemsToTimeline(threadID: result.threadID, items: result.items)
    updatePendingApprovals(threadID: result.threadID, approvals: result.pendingApprovals)
    await refreshMemoryState()
    await loadThreadHistory(threadID: result.threadID)
  }
}
