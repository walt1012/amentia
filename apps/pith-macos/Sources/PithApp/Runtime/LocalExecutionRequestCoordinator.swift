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

  func clearAll() {
    agentRequest.cancelAndClear()
    approvalExecution.cancelAndClear()
  }
}

private final class PendingRuntimeRequestState {
  private(set) var requestID: UUID?
  private(set) var threadID: String?
  private var cancellationRequested = false
  private var task: Task<Void, Never>?

  var isPending: Bool {
    requestID != nil
  }

  var canCancel: Bool {
    requestID != nil && threadID != nil && !cancellationRequested
  }

  func begin(threadID: String) -> UUID {
    cancelAndClear()
    let requestID = UUID()
    self.requestID = requestID
    self.threadID = threadID
    cancellationRequested = false
    return requestID
  }

  func bind(task: Task<Void, Never>, requestID: UUID) {
    guard self.requestID == requestID else {
      return
    }

    self.task = task
  }

  func clear(requestID: UUID) {
    guard self.requestID == requestID else {
      return
    }

    clear()
  }

  func clear() {
    requestID = nil
    threadID = nil
    cancellationRequested = false
    task = nil
  }

  func requestCancellation() -> String? {
    guard canCancel, let threadID else {
      return nil
    }

    cancellationRequested = true
    return threadID
  }

  func cancelAndClear() {
    task?.cancel()
    clear()
  }
}
