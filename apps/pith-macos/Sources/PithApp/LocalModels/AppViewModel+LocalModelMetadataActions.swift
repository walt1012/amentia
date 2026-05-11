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
      runtimeDetail = "Launch the runtime before preparing local model metadata."
      return
    }
    guard !modelDownloadCoordinator.isDownloading else {
      runtimeDetail = "Finish, pause, or cancel the current model download before preparing metadata."
      return
    }
    guard let requestToken = localModelMetadataCoordinator.begin() else {
      runtimeDetail = "Model metadata preparation is already running."
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
          ? "Pack metadata was already present."
          : "Prepared \(result.copiedFiles.count) local model metadata file(s)."
        runtimeDetail = "\(copiedSummary) Manifest: \(result.manifestPath)"
      } catch {
        guard !Task.isCancelled,
              localModelMetadataCoordinator.isCurrent(requestToken)
        else {
          return
        }
        runtimeDetail = "Model metadata bootstrap failed: \(error.localizedDescription)"
      }
    }
    localModelMetadataCoordinator.bind(task: task, token: requestToken)
  }
}

struct LocalModelMetadataRequestToken: Equatable {
  fileprivate let id: UUID
}

final class LocalModelMetadataCoordinator {
  private var activeRequestID: UUID?
  private var activeTask: Task<Void, Never>?

  var isRunning: Bool {
    activeRequestID != nil
  }

  func begin() -> LocalModelMetadataRequestToken? {
    guard activeRequestID == nil else {
      return nil
    }

    let requestID = UUID()
    activeRequestID = requestID
    return LocalModelMetadataRequestToken(id: requestID)
  }

  func bind(task: Task<Void, Never>, token: LocalModelMetadataRequestToken) {
    guard isCurrent(token) else {
      task.cancel()
      return
    }

    activeTask = task
  }

  func isCurrent(_ token: LocalModelMetadataRequestToken) -> Bool {
    activeRequestID == token.id
  }

  func finish(_ token: LocalModelMetadataRequestToken) {
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
