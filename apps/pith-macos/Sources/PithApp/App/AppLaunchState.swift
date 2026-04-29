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
    let localModels = LocalModelCatalog.summaries(
      storageRootPath: runtimeBridge.localModelStorageRootPath(),
      activeModelPath: runtimeBridge.activeLocalModelPath()
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
    let runtimeDetail = pausedDownload == nil
      ? "Runtime not launched"
      : "Runtime not launched | paused model download available"

    return AppLaunchState(
      welcomeState: welcomeState,
      localModels: localModels,
      selectedSetupModelID: selectedSetupModelID,
      pausedDownload: pausedDownload,
      modelDownloadProgress: modelDownloadProgress,
      runtimeDetail: runtimeDetail
    )
  }
}
