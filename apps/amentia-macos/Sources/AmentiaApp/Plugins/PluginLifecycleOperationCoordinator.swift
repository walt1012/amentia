import Foundation

final class PluginLifecycleOperationCoordinator {
  private let taskSlot = CancellableTaskSlot()

  var isActive: Bool {
    taskSlot.isActive
  }

  func begin() -> UUID? {
    taskSlot.begin()
  }

  func bind(task: Task<Void, Never>, operationID: UUID) {
    taskSlot.bind(task: task, requestID: operationID)
  }

  func isCurrent(_ operationID: UUID) -> Bool {
    taskSlot.isCurrent(operationID)
  }

  func finish(_ operationID: UUID) {
    taskSlot.finish(operationID)
  }

  func cancel() {
    taskSlot.cancel()
  }
}
