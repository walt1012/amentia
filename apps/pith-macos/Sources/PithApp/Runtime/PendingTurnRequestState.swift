import Foundation

final class PendingTurnRequestState {
  private(set) var requestID: UUID?
  private(set) var threadID: String?
  private var task: Task<Void, Never>?

  var isPending: Bool {
    requestID != nil
  }

  var canCancel: Bool {
    requestID != nil && threadID != nil
  }

  func begin(threadID: String) -> UUID {
    task?.cancel()
    clear()
    let requestID = UUID()
    self.requestID = requestID
    self.threadID = threadID
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
    task = nil
  }
}
