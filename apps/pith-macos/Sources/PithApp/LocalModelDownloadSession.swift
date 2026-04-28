import Foundation

struct LocalModelDownloadSessionStartState {
  let activeModelID: String
  let pausedModelID: String?
  let progress: ModelDownloadProgress
  let clearsPausedState: Bool
  let shouldActivateAfterDownload: Bool
}

enum LocalModelDownloadSessionPlanner {
  static func startState(
    model: LocalModelSummary,
    startPlan: LocalModelDownloadStartPlan,
    activateAfterDownload: Bool,
    isLocalModelReady: Bool
  ) -> LocalModelDownloadSessionStartState {
    LocalModelDownloadSessionStartState(
      activeModelID: model.id,
      pausedModelID: nil,
      progress: startPlan.progress,
      clearsPausedState: true,
      shouldActivateAfterDownload: activateAfterDownload || !isLocalModelReady
    )
  }
}
