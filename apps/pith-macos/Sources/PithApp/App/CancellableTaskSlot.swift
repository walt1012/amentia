import Foundation

final class CancellableTaskSlot {
  private var requestID: UUID?
  private var task: Task<Void, Never>?

  var isActive: Bool {
    requestID != nil
  }

  func begin() -> UUID? {
    guard requestID == nil else {
      return nil
    }

    let nextRequestID = UUID()
    requestID = nextRequestID
    return nextRequestID
  }

  func replace() -> UUID {
    cancel()
    let nextRequestID = UUID()
    requestID = nextRequestID
    return nextRequestID
  }

  func bind(task: Task<Void, Never>, requestID: UUID) {
    guard isCurrent(requestID) else {
      task.cancel()
      return
    }

    self.task = task
  }

  func isCurrent(_ requestID: UUID) -> Bool {
    self.requestID == requestID
  }

  func finish(_ requestID: UUID) {
    guard isCurrent(requestID) else {
      return
    }

    clear()
  }

  func cancel() {
    task?.cancel()
    clear()
  }

  func clear() {
    requestID = nil
    task = nil
  }
}
