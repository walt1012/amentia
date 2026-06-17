import Foundation

struct WorkspaceOpenBootstrap {
  let workspace: RuntimeBridge.RuntimeWorkspace
  let memoryRefresh: MemoryStateRefresh
  let threadList: [RuntimeBridge.RuntimeThreadSummary]
}

enum WorkspaceOpenBootstrapLoader {
  static func load(
    runtimeBridge: RuntimeBridge,
    path: String
  ) async throws -> WorkspaceOpenBootstrap {
    let workspace = try await runtimeBridge.openWorkspace(path: path)
    let memoryRefresh = await MemoryStateLoader.refresh(using: runtimeBridge)
    let threadList = try await runtimeBridge.listThreads()

    return WorkspaceOpenBootstrap(
      workspace: workspace,
      memoryRefresh: memoryRefresh,
      threadList: threadList
    )
  }
}

struct WorkspaceOpenRequestToken: Equatable {
  fileprivate let id: UUID
  let previousRuntimeDetail: String
}

final class WorkspaceOpenCoordinator {
  private let taskSlot = CancellableTaskSlot()

  var isOpening: Bool {
    taskSlot.isActive
  }

  func begin(previousRuntimeDetail: String) -> WorkspaceOpenRequestToken? {
    guard let requestID = taskSlot.begin() else {
      return nil
    }

    return WorkspaceOpenRequestToken(
      id: requestID,
      previousRuntimeDetail: previousRuntimeDetail
    )
  }

  func bind(task: Task<Void, Never>, token: WorkspaceOpenRequestToken) {
    taskSlot.bind(task: task, requestID: token.id)
  }

  func isCurrent(_ token: WorkspaceOpenRequestToken) -> Bool {
    taskSlot.isCurrent(token.id)
  }

  func finish(_ token: WorkspaceOpenRequestToken) {
    taskSlot.finish(token.id)
  }

  func cancel() {
    taskSlot.cancel()
  }
}
