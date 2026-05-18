import Foundation

final class LocalModelActivationCoordinator {
  private let taskSlot = CancellableTaskSlot()

  var isActivating: Bool {
    taskSlot.isActive
  }

  func begin() -> UUID? {
    taskSlot.begin()
  }

  func bind(task: Task<Void, Never>, requestID: UUID) {
    taskSlot.bind(task: task, requestID: requestID)
  }

  func isCurrent(_ requestID: UUID) -> Bool {
    taskSlot.isCurrent(requestID)
  }

  func finish(_ requestID: UUID) {
    taskSlot.finish(requestID)
  }

  func cancel() {
    taskSlot.cancel()
  }
}
