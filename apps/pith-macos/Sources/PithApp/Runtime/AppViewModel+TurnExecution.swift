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
        localExecutionRequests.clearAgentRequest(requestID: requestID)
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
    localExecutionRequests.bindAgentRequest(task: task, requestID: requestID)
  }

  func respondToApproval(approvalID: String, decision: String) {
    guard canRespondToApproval(approvalID: approvalID),
          decision == "approved" || decision == "denied"
    else {
      return
    }

    guard !localExecutionRequests.isApprovalExecutionPending else {
      return
    }

    let approvalThreadID = self.selectedThreadID
    let requestID = approvalThreadID.map {
      localExecutionRequests.beginApprovalExecution(threadID: $0)
    }

    let task = Task {
      defer {
        if let requestID {
          localExecutionRequests.clearApprovalExecution(requestID: requestID)
        }
      }
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
    if let requestID {
      localExecutionRequests.bindApprovalExecution(task: task, requestID: requestID)
    }
  }

  func cancelActiveTurn() {
    guard canCancelActiveTurn() else {
      return
    }

    if let pendingThreadID = requestPendingTurnCancellation() {
      Task {
        do {
          _ = try await runtimeBridge.cancelRunningTurn(threadID: pendingThreadID)
        } catch {
          appendEntry(
            to: pendingThreadID,
            TimelineEventPresenter.turnCancelFailed(error: error)
          )
        }
      }
      return
    }

    if let approvalThreadID = requestPendingApprovalCancellation() {
      Task {
        do {
          _ = try await runtimeBridge.cancelRunningTurn(threadID: approvalThreadID)
        } catch {
          appendEntry(
            to: approvalThreadID,
            TimelineEventPresenter.turnCancelFailed(error: error)
          )
        }
      }
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
    activeTurnID != nil || localExecutionRequests.hasPendingExecution
  }

  func hasCancelableLocalExecution() -> Bool {
    timelineState.hasCancelableRuntimeTurn || localExecutionRequests.canCancelPendingExecution
  }

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
    if wasCancelled {
      runtimeDetail = TimelineEventPresenter.pendingTurnCancelledDetail
    }
    let preview = wasCancelled
      ? TimelineEventPresenter.cancelledResponsePreview
      : TimelineEventPresenter.turnPreview(
        turnID: result.turnID,
        activeTurnID: result.activeTurnID
      )
    refreshThreadPreview(
      threadID: result.threadID,
      preview: preview
    )

    if refreshMemory {
      await refreshMemoryState()
    }
  }

  private func beginPendingLocalTurn(threadID: String) -> UUID {
    draftMessage = ""
    runtimeDetail = TimelineEventPresenter.generatingLocalResponseDetail
    return localExecutionRequests.beginAgentRequest(threadID: threadID)
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

  private func requestPendingTurnCancellation() -> String? {
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

  private func requestPendingApprovalCancellation() -> String? {
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
