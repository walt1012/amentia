import Foundation

struct AppLaunchState {
  let welcomeState: WelcomeTimelineState
  let localModels: [LocalModelSummary]
  let selectedSetupModelID: String
  let pausedDownload: PersistedModelDownload?
  let modelDownloadProgress: ModelDownloadProgress?
  let runtimeDetail: String

  static func make(runtimeBridge: RuntimeBridge) -> Self {
    let welcomeState = TimelineSessionState.welcomeState()
    let activeModelPath = runtimeBridge.activeLocalModelPath()
    let activeModelInvalidationDetail = runtimeBridge.consumeActiveLocalModelInvalidationDetail()
    let localModels = LocalModelCatalog.summaries(
      storageRootPath: runtimeBridge.localModelStorageRootPath(),
      activeModelPath: activeModelPath
    )
    let pausedDownload = LocalModelCatalog.loadPausedDownload(matching: localModels)
    let selectedSetupModelID =
      pausedDownload?.modelID
      ?? AppPreferences.storedSelectedSetupModelID(matching: localModels)
      ?? LocalModelCatalog.defaultFirstUseModelID
    let modelDownloadProgress = LocalModelCatalog.restoredProgress(
      from: pausedDownload,
      localModels: localModels
    )
    let runtimeDetail = launchRuntimeDetail(
      pausedDownload: pausedDownload,
      activeModelInvalidationDetail: activeModelInvalidationDetail
    )

    return AppLaunchState(
      welcomeState: welcomeState,
      localModels: localModels,
      selectedSetupModelID: selectedSetupModelID,
      pausedDownload: pausedDownload,
      modelDownloadProgress: modelDownloadProgress,
      runtimeDetail: runtimeDetail
    )
  }

  private static func launchRuntimeDetail(
    pausedDownload: PersistedModelDownload?,
    activeModelInvalidationDetail: String?
  ) -> String {
    var details = ["Runtime not launched"]
    if pausedDownload != nil {
      details.append("paused model download available")
    }
    if let activeModelInvalidationDetail {
      details.append(activeModelInvalidationDetail)
    }
    return details.joined(separator: " | ")
  }
}
