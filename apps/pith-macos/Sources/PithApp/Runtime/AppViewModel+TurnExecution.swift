import Foundation

@MainActor
extension AppViewModel {
  func sendDraftMessage() {
    guard let draftTurn = SessionActionPlanner.preparedDraftTurn(
      snapshot: sessionActionSnapshot(),
      selectedThreadID: selectedThreadID,
      draftMessage: draftMessage
    ) else {
      return
    }

    let threadID = draftTurn.threadID
    let message = draftTurn.message
    let requestID = beginPendingLocalTurn(threadID: threadID)

    let task = Task {
      defer {
        pendingTurnRequest.clear(requestID: requestID)
      }
      do {
        let result = try await runtimeBridge.startTurn(threadID: threadID, message: message)
        await applyRuntimeTurnResult(result)
      } catch {
        if Task.isCancelled {
          applyPendingTurnCancellation(threadID: threadID)
          return
        }
        applyPendingTurnFailure(threadID: threadID, message: message, error: error)
      }
    }
    pendingTurnRequest.bind(task: task, requestID: requestID)
  }

  func respondToApproval(approvalID: String, decision: String) {
    guard canRespondToApproval(approvalID: approvalID),
          decision == "approved" || decision == "denied"
    else {
      return
    }

    Task {
      do {
        let result = try await runtimeBridge.respondToApproval(
          approvalID: approvalID,
          decision: decision
        )
        await applyApprovalResponse(result)
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.approvalResponseFailed(error: error)
        )
      }
    }
  }

  func cancelActiveTurn() {
    guard canCancelActiveTurn() else {
      return
    }

    if cancelPendingTurnRequest() {
      return
    }

    guard let activeTurnID,
          let activeTurnThreadID = timelineState.activeTurnThreadID
    else {
      return
    }

    Task {
      do {
        let result = try await runtimeBridge.cancelTurn(turnID: activeTurnID)
        await applyTurnCancellation(result, previewThreadID: activeTurnThreadID)
      } catch {
        appendEntry(
          to: activeTurnThreadID,
          TimelineEventPresenter.turnCancelFailed(error: error)
        )
      }
    }
  }

  func hasActiveOrPendingTurn() -> Bool {
    activeTurnID != nil || pendingTurnRequest.isPending
  }

  func applyRuntimeTurnResult(
    _ result: RuntimeBridge.RuntimeTurnResult,
    refreshMemory: Bool = false
  ) async {
    appendItemsToTimeline(threadID: result.threadID, items: result.items)
    updatePendingApprovals(threadID: result.threadID, approvals: result.pendingApprovals)
    updateActiveTurn(threadID: result.threadID, activeTurnID: result.activeTurnID)
    refreshThreadPreview(
      threadID: result.threadID,
      preview: TimelineEventPresenter.turnPreview(
        turnID: result.turnID,
        activeTurnID: result.activeTurnID
      )
    )

    if refreshMemory {
      await refreshMemoryState()
    }
  }

  private func beginPendingLocalTurn(threadID: String) -> UUID {
    draftMessage = ""
    runtimeDetail = TimelineEventPresenter.generatingLocalResponseDetail
    return pendingTurnRequest.begin(threadID: threadID)
  }

  private func applyPendingTurnCancellation(threadID: String) {
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

  private func applyPendingTurnFailure(
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

  private func applyApprovalResponse(_ result: RuntimeBridge.RuntimeApprovalResponse) async {
    appendItemsToTimeline(threadID: result.threadID, items: result.items)
    updatePendingApprovals(threadID: result.threadID, approvals: result.pendingApprovals)
    await refreshMemoryState()
    await loadThreadHistory(threadID: result.threadID)
  }

  private func applyTurnCancellation(
    _ result: RuntimeBridge.RuntimeTurnCancellation,
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

  private func cancelPendingTurnRequest() -> Bool {
    guard let threadID = pendingTurnRequest.cancel() else {
      return false
    }

    runtimeDetail = TimelineEventPresenter.cancellingTurnDetail
    refreshThreadPreview(
      threadID: threadID,
      preview: TimelineEventPresenter.cancellingResponsePreview
    )
    return true
  }
}
