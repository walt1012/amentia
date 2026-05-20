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

  func requestPendingTurnCancellation() -> PendingLocalExecutionCancellation? {
    guard let cancellation = localExecutionRequests.requestLocalWorkCancellation() else {
      return nil
    }

    runtimeDetail = pendingCancellationDetail(cancellation.kind)
    refreshThreadPreview(
      threadID: cancellation.threadID,
      preview: pendingCancellationPreview(cancellation.kind)
    )
    return cancellation
  }

  func requestPendingApprovalCancellation() -> String? {
    guard let threadID = localExecutionRequests.requestApprovalCancellationThreadID() else {
      return nil
    }

    runtimeDetail = TimelineEventPresenter.cancellingTurnDetail
    refreshThreadPreview(
      threadID: threadID,
      preview: TimelineEventPresenter.cancellingResponsePreview
    )
    return threadID
  }

  private func pendingCancellationDetail(_ kind: PendingLocalExecutionKind) -> String {
    switch kind {
    case .agentTurn, .approvalExecution:
      return TimelineEventPresenter.cancellingTurnDetail
    case .pluginCommand:
      return TimelineEventPresenter.cancellingPluginCommandDetail
    }
  }

  private func pendingCancellationPreview(_ kind: PendingLocalExecutionKind) -> String {
    switch kind {
    case .agentTurn, .approvalExecution:
      return TimelineEventPresenter.cancellingResponsePreview
    case .pluginCommand:
      return TimelineEventPresenter.cancellingPluginCommandPreview
    }
  }
}

struct TurnCancellationRequestToken: Equatable {
  fileprivate let id: UUID
}

final class TurnCancellationCoordinator {
  private let taskSlot = CancellableTaskSlot()

  var isCancelling: Bool {
    taskSlot.isActive
  }

  func begin() -> TurnCancellationRequestToken? {
    guard let requestID = taskSlot.begin() else {
      return nil
    }

    return TurnCancellationRequestToken(id: requestID)
  }

  func bind(task: Task<Void, Never>, token: TurnCancellationRequestToken) {
    taskSlot.bind(task: task, requestID: token.id)
  }

  func isCurrent(_ token: TurnCancellationRequestToken) -> Bool {
    taskSlot.isCurrent(token.id)
  }

  func finish(_ token: TurnCancellationRequestToken) {
    taskSlot.finish(token.id)
  }

  func cancel() {
    taskSlot.cancel()
  }
}
