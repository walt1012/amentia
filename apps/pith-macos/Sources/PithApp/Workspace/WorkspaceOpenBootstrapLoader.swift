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
}

final class WorkspaceOpenCoordinator {
  private var activeRequestID: UUID?

  var isOpening: Bool {
    activeRequestID != nil
  }

  func begin() -> WorkspaceOpenRequestToken? {
    guard activeRequestID == nil else {
      return nil
    }

    let requestID = UUID()
    activeRequestID = requestID
    return WorkspaceOpenRequestToken(id: requestID)
  }

  func isCurrent(_ token: WorkspaceOpenRequestToken) -> Bool {
    activeRequestID == token.id
  }

  func finish(_ token: WorkspaceOpenRequestToken) {
    guard isCurrent(token) else {
      return
    }

    activeRequestID = nil
  }

  func cancel() {
    activeRequestID = nil
  }
}
