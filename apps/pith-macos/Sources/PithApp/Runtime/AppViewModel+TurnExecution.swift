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
        localExecutionRequests.clearLocalWorkRequest(requestID: requestID)
      }
      do {
        let result = try await runtimeBridge.startTurn(
          threadID: threadID,
          message: message,
          localExecutionSafetyMode: selectedLocalExecutionSafetyMode
        )
        await applyRuntimeTurnResult(result)
      } catch {
        if Task.isCancelled {
          applyPendingTurnCancellation(threadID: threadID)
          return
        }
        applyPendingTurnFailure(threadID: threadID, message: message, error: error)
      }
    }
    localExecutionRequests.bindLocalWorkRequest(task: task, requestID: requestID)
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
        runtimeDetail = TimelineEventPresenter.approvalResponseFailedDetail(error: error)
        appendEntry(
          to: approvalThreadID ?? selectedThreadID,
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

    if let pendingCancellation = requestPendingTurnCancellation() {
      Task {
        do {
          _ = try await runtimeBridge.cancelRunningExecution(threadID: pendingCancellation.threadID)
        } catch {
          localExecutionRequests.restoreLocalWorkCancellationRequest(
            threadID: pendingCancellation.threadID
          )
          runtimeDetail = TimelineEventPresenter.turnCancelFailedDetail(error: error)
          appendEntry(
            to: pendingCancellation.threadID,
            TimelineEventPresenter.turnCancelFailed(error: error)
          )
        }
      }
      return
    }

    if let approvalThreadID = requestPendingApprovalCancellation() {
      Task {
        do {
          _ = try await runtimeBridge.cancelRunningExecution(threadID: approvalThreadID)
        } catch {
          localExecutionRequests.restoreApprovalCancellationRequest(threadID: approvalThreadID)
          runtimeDetail = TimelineEventPresenter.turnCancelFailedDetail(error: error)
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

    guard let requestToken = turnCancellationCoordinator.begin() else {
      return
    }
    runtimeDetail = TimelineEventPresenter.cancellingTurnDetail
    refreshThreadPreview(
      threadID: activeTurnThreadID,
      preview: TimelineEventPresenter.cancellingResponsePreview
    )

    let task = Task {
      defer {
        turnCancellationCoordinator.finish(requestToken)
      }
      do {
        let result = try await runtimeBridge.cancelTurn(turnID: activeTurnID)
        guard turnCancellationCoordinator.isCurrent(requestToken) else {
          return
        }
        await applyTurnCancellation(result, previewThreadID: activeTurnThreadID)
      } catch {
        guard !Task.isCancelled,
              turnCancellationCoordinator.isCurrent(requestToken)
        else {
          return
        }
        runtimeDetail = TimelineEventPresenter.turnCancelFailedDetail(error: error)
        appendEntry(
          to: activeTurnThreadID,
          TimelineEventPresenter.turnCancelFailed(error: error)
        )
      }
    }
    turnCancellationCoordinator.bind(task: task, token: requestToken)
  }

  func hasActiveOrPendingTurn() -> Bool {
    activeTurnID != nil || localExecutionRequests.hasPendingExecution
  }

  func hasCancelableLocalExecution() -> Bool {
    (timelineState.hasCancelableRuntimeTurn && !turnCancellationCoordinator.isCancelling)
      || localExecutionRequests.canCancelPendingExecution
  }
}
