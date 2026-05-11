import Foundation

final class LocalExecutionRequestCoordinator {
  private let agentRequest = PendingRuntimeRequestState()
  private let approvalExecution = PendingRuntimeRequestState()

  var hasPendingExecution: Bool {
    agentRequest.isPending || approvalExecution.isPending
  }

  var canCancelPendingExecution: Bool {
    agentRequest.canCancel || approvalExecution.canCancel
  }

  var isApprovalExecutionPending: Bool {
    approvalExecution.isPending
  }

  func beginAgentRequest(threadID: String) -> UUID {
    agentRequest.begin(threadID: threadID)
  }

  func bindAgentRequest(task: Task<Void, Never>, requestID: UUID) {
    agentRequest.bind(task: task, requestID: requestID)
  }

  func clearAgentRequest(requestID: UUID) {
    agentRequest.clear(requestID: requestID)
  }

  func requestAgentCancellationThreadID() -> String? {
    agentRequest.requestCancellation()
  }

  func restoreAgentCancellationRequest(threadID: String) {
    agentRequest.restoreCancellationRequest(threadID: threadID)
  }

  func beginApprovalExecution(threadID: String) -> UUID {
    approvalExecution.begin(threadID: threadID)
  }

  func bindApprovalExecution(task: Task<Void, Never>, requestID: UUID) {
    approvalExecution.bind(task: task, requestID: requestID)
  }

  func clearApprovalExecution(requestID: UUID) {
    approvalExecution.clear(requestID: requestID)
  }

  func requestApprovalCancellationThreadID() -> String? {
    approvalExecution.requestCancellation()
  }

  func restoreApprovalCancellationRequest(threadID: String) {
    approvalExecution.restoreCancellationRequest(threadID: threadID)
  }

  func clearAll() {
    agentRequest.cancelAndClear()
    approvalExecution.cancelAndClear()
  }
}

private final class PendingRuntimeRequestState {
  private let taskSlot = CancellableTaskSlot()
  private(set) var threadID: String?
  private var cancellationRequested = false

  var isPending: Bool {
    taskSlot.isActive
  }

  var canCancel: Bool {
    taskSlot.isActive && threadID != nil && !cancellationRequested
  }

  func begin(threadID: String) -> UUID {
    let requestID = taskSlot.replace()
    self.threadID = threadID
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
    cancellationRequested = false
  }

  func requestCancellation() -> String? {
    guard canCancel, let threadID else {
      return nil
    }

    cancellationRequested = true
    return threadID
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
