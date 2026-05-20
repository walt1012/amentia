import Foundation

enum PendingLocalExecutionKind {
  case agentTurn
  case approvalExecution
  case pluginCommand
}

struct PendingLocalExecutionCancellation {
  let threadID: String
  let kind: PendingLocalExecutionKind
}

final class LocalExecutionRequestCoordinator {
  private let localWork = PendingRuntimeRequestState()
  private let approvalExecution = PendingRuntimeRequestState()

  var hasPendingExecution: Bool {
    localWork.isPending || approvalExecution.isPending
  }

  var canCancelPendingExecution: Bool {
    localWork.canCancel || approvalExecution.canCancel
  }

  var isApprovalExecutionPending: Bool {
    approvalExecution.isPending
  }

  func beginAgentTurnRequest(threadID: String) -> UUID {
    localWork.begin(threadID: threadID, kind: .agentTurn)
  }

  func beginPluginCommandRequest(threadID: String) -> UUID {
    localWork.begin(threadID: threadID, kind: .pluginCommand)
  }

  func bindLocalWorkRequest(task: Task<Void, Never>, requestID: UUID) {
    localWork.bind(task: task, requestID: requestID)
  }

  func clearLocalWorkRequest(requestID: UUID) {
    localWork.clear(requestID: requestID)
  }

  func requestLocalWorkCancellation() -> PendingLocalExecutionCancellation? {
    localWork.requestCancellation()
  }

  func restoreLocalWorkCancellationRequest(threadID: String) {
    localWork.restoreCancellationRequest(threadID: threadID)
  }

  func beginApprovalExecution(threadID: String) -> UUID {
    approvalExecution.begin(threadID: threadID, kind: .approvalExecution)
  }

  func bindApprovalExecution(task: Task<Void, Never>, requestID: UUID) {
    approvalExecution.bind(task: task, requestID: requestID)
  }

  func clearApprovalExecution(requestID: UUID) {
    approvalExecution.clear(requestID: requestID)
  }

  func requestApprovalCancellationThreadID() -> String? {
    approvalExecution.requestCancellation()?.threadID
  }

  func restoreApprovalCancellationRequest(threadID: String) {
    approvalExecution.restoreCancellationRequest(threadID: threadID)
  }

  func clearAll() {
    localWork.cancelAndClear()
    approvalExecution.cancelAndClear()
  }
}

private final class PendingRuntimeRequestState {
  private let taskSlot = CancellableTaskSlot()
  private(set) var threadID: String?
  private var kind: PendingLocalExecutionKind?
  private var cancellationRequested = false

  var isPending: Bool {
    taskSlot.isActive
  }

  var canCancel: Bool {
    taskSlot.isActive && threadID != nil && !cancellationRequested
  }

  func begin(threadID: String, kind: PendingLocalExecutionKind) -> UUID {
    let requestID = taskSlot.replace()
    self.threadID = threadID
    self.kind = kind
    cancellationRequested = false
    return requestID
  }

  func bind(task: Task<Void, Never>, requestID: UUID) {
    taskSlot.bind(task: task, requestID: requestID)
  }

  func clear(requestID: UUID) {
    guard taskSlot.isCurrent(requestID) else {
      return
    }

    clear()
  }

  func clear() {
    taskSlot.clear()
    clearMetadata()
  }

  private func clearMetadata() {
    threadID = nil
    kind = nil
    cancellationRequested = false
  }

  func requestCancellation() -> PendingLocalExecutionCancellation? {
    guard canCancel, let threadID, let kind else {
      return nil
    }

    cancellationRequested = true
    return PendingLocalExecutionCancellation(threadID: threadID, kind: kind)
  }

  func restoreCancellationRequest(threadID: String) {
    guard self.threadID == threadID else {
      return
    }

    cancellationRequested = false
  }

  func cancelAndClear() {
    taskSlot.cancel()
    clearMetadata()
  }
}
