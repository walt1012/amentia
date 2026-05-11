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
  private var activeRequestID: UUID?
  private var activeTask: Task<Void, Never>?

  var isOpening: Bool {
    activeRequestID != nil
  }

  func begin(previousRuntimeDetail: String) -> WorkspaceOpenRequestToken? {
    guard activeRequestID == nil else {
      return nil
    }

    let requestID = UUID()
    activeRequestID = requestID
    return WorkspaceOpenRequestToken(
      id: requestID,
      previousRuntimeDetail: previousRuntimeDetail
    )
  }

  func bind(task: Task<Void, Never>, token: WorkspaceOpenRequestToken) {
    guard isCurrent(token) else {
      task.cancel()
      return
    }

    activeTask = task
  }

  func isCurrent(_ token: WorkspaceOpenRequestToken) -> Bool {
    activeRequestID == token.id
  }

  func finish(_ token: WorkspaceOpenRequestToken) {
    guard isCurrent(token) else {
      return
    }

    clear()
  }

  func cancel() {
    activeTask?.cancel()
    clear()
  }

  private func clear() {
    activeRequestID = nil
    activeTask = nil
  }
}
