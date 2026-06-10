import Foundation

enum LocalModelDownloadInterruptionMode {
  case paused(resumeData: Data)
  case cancelled
  case failed
}

struct LocalModelDownloadInterruptionPlan {
  let mode: LocalModelDownloadInterruptionMode
  let runtimeDetail: String
  let timelineTitle: String
  let timelineBody: String
  let timelineKind: TimelineEntry.Kind
  let attributes: [String: String]
  let clearsPausedState: Bool
  let clearsProgress: Bool
  let removesPartialFile: Bool
}

enum LocalModelDownloadInterruptionPlanner {
  static func plan(model: LocalModelSummary, error: Error) -> LocalModelDownloadInterruptionPlan {
    let modelName = LocalModelDisplayPresenter.actionName(model)
    if let paused = error as? ModelDownloadPaused {
      return LocalModelDownloadInterruptionPlan(
        mode: .paused(resumeData: paused.resumeData),
        runtimeDetail: "Paused \(modelName) download. Continue to resume from the saved partial state.",
        timelineTitle: "Model Download Paused",
        timelineBody:
          "\(modelName) download was paused and can continue from the saved local state.",
        timelineKind: .system,
        attributes: [
          "result": "paused",
        ],
        clearsPausedState: false,
        clearsProgress: false,
        removesPartialFile: false
      )
    }

    if isCancellation(error) {
      return cancellationPlan(model: model)
    }

    return LocalModelDownloadInterruptionPlan(
      mode: .failed,
      runtimeDetail: "Model download failed: \(error.localizedDescription)",
      timelineTitle: "Model Download Failed",
      timelineBody: "\(modelName) download failed: \(error.localizedDescription)",
      timelineKind: .warning,
      attributes: [
        "error": error.localizedDescription,
        "result": "failed",
      ],
      clearsPausedState: true,
      clearsProgress: true,
      removesPartialFile: false
    )
  }

  static func cancellationPlan(model: LocalModelSummary) -> LocalModelDownloadInterruptionPlan {
    let modelName = LocalModelDisplayPresenter.actionName(model)
    LocalModelDownloadInterruptionPlan(
      mode: .cancelled,
      runtimeDetail: "Cancelled \(modelName) download and cleared partial state.",
      timelineTitle: "Model Download Cancelled",
      timelineBody: "\(modelName) download was cancelled and the partial file was cleared.",
      timelineKind: .system,
      attributes: [
        "result": "cancelled",
      ],
      clearsPausedState: true,
      clearsProgress: true,
      removesPartialFile: true
    )
  }

  private static func isCancellation(_ error: Error) -> Bool {
    if error is CancellationError {
      return true
    }

    return (error as? URLError)?.code == .cancelled
  }
}

enum LocalModelDownloadCancelMode {
  case running
  case paused(model: LocalModelSummary)
  case orphanedPaused(modelID: String)
}

struct LocalModelDownloadCancelPlan {
  let mode: LocalModelDownloadCancelMode
  let runtimeDetail: String
}

enum LocalModelDownloadControlPlanner {
  static func pauseDetail(activeModelID: String?, models: [LocalModelSummary]) -> String {
    "Pausing \(displayName(for: activeModelID, models: models)) download..."
  }

  static func cancelPlan(
    isDownloading: Bool,
    activeModelID: String?,
    pausedModelID: String?,
    models: [LocalModelSummary]
  ) -> LocalModelDownloadCancelPlan? {
    if isDownloading {
      return LocalModelDownloadCancelPlan(
        mode: .running,
        runtimeDetail: "Cancelling \(displayName(for: activeModelID, models: models)) download..."
      )
    }

    guard let pausedModelID else {
      return nil
    }

    guard let model = models.first(where: { $0.id == pausedModelID }) else {
      return LocalModelDownloadCancelPlan(
        mode: .orphanedPaused(modelID: pausedModelID),
        runtimeDetail: "Cancelled model download and cleared partial state."
      )
    }

    return LocalModelDownloadCancelPlan(
      mode: .paused(model: model),
      runtimeDetail: LocalModelDownloadInterruptionPlanner
        .cancellationPlan(model: model)
        .runtimeDetail
    )
  }

  private static func displayName(for modelID: String?, models: [LocalModelSummary]) -> String {
    modelID
      .flatMap { id in models.first(where: { $0.id == id }).map(LocalModelDisplayPresenter.actionName) }
      ?? "model"
  }
}

struct LocalModelDownloadProgressUpdate {
  let modelID: String
  let activeModelID: String?
  let currentProgress: ModelDownloadProgress?
  let bytesReceived: Int64
  let totalBytes: Int64
  let updatedAt: Date
}

enum LocalModelDownloadProgressUpdater {
  static func updatedProgress(
    _ update: LocalModelDownloadProgressUpdate
  ) -> ModelDownloadProgress? {
    guard update.activeModelID == update.modelID,
          update.currentProgress?.modelID == update.modelID,
          var progress = update.currentProgress
    else {
      return nil
    }

    progress.bytesReceived = max(update.bytesReceived, progress.bytesReceived)
    progress.totalBytes = update.totalBytes > 0 ? update.totalBytes : progress.totalBytes
    progress.updatedAt = update.updatedAt
    return progress
  }
}
