import Foundation

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
        runtimeDetail: "Cancelled local model download and cleared partial state."
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
      .flatMap { id in models.first(where: { $0.id == id })?.displayName }
      ?? "local model"
  }
}
