import Foundation

@MainActor
extension AppViewModel {
  func canBootstrapModelPackMetadata() -> Bool {
    runtimeState == .ready
      && !modelDownloadCoordinator.isDownloading
      && !localModelMetadataCoordinator.isRunning
  }

  func bootstrapModelPackMetadata() {
    guard runtimeState == .ready else {
      runtimeDetail = "Launch the local engine before repairing model setup."
      return
    }
    guard !modelDownloadCoordinator.isDownloading else {
      runtimeDetail = "Finish, pause, or cancel the current model download before repairing setup."
      return
    }
    guard let requestToken = localModelMetadataCoordinator.begin() else {
      runtimeDetail = "Model setup repair is already running."
      return
    }

    let task = Task {
      defer {
        localModelMetadataCoordinator.finish(requestToken)
      }
      do {
        let result = try await runtimeBridge.bootstrapModelPack()
        guard localModelMetadataCoordinator.isCurrent(requestToken) else {
          return
        }
        await refreshModelHealthState()
        let copiedSummary = result.copiedFiles.isEmpty
          ? "Model setup info was already present."
          : "Prepared \(result.copiedFiles.count) local model setup file(s)."
        runtimeDetail = "\(copiedSummary) Local model setup is ready."
      } catch {
        guard !Task.isCancelled,
              localModelMetadataCoordinator.isCurrent(requestToken)
        else {
          return
        }
        runtimeDetail = "Model setup repair failed: \(error.localizedDescription)"
      }
    }
    localModelMetadataCoordinator.bind(task: task, token: requestToken)
  }
}

struct LocalModelMetadataRequestToken: Equatable {
  fileprivate let id: UUID
}

final class LocalModelMetadataCoordinator {
  private let taskSlot = CancellableTaskSlot()

  var isRunning: Bool {
    taskSlot.isActive
  }

  func begin() -> LocalModelMetadataRequestToken? {
    guard let requestID = taskSlot.begin() else {
      return nil
    }

    return LocalModelMetadataRequestToken(id: requestID)
  }

  func bind(task: Task<Void, Never>, token: LocalModelMetadataRequestToken) {
    taskSlot.bind(task: task, requestID: token.id)
  }

  func isCurrent(_ token: LocalModelMetadataRequestToken) -> Bool {
    taskSlot.isCurrent(token.id)
  }

  func finish(_ token: LocalModelMetadataRequestToken) {
    taskSlot.finish(token.id)
  }

  func cancel() {
    taskSlot.cancel()
  }
}
