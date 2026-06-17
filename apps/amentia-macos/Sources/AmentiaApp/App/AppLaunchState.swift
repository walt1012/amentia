import Foundation

struct AppLaunchState {
  let welcomeState: WelcomeTimelineState
  let localModels: [LocalModelSummary]
  let selectedSetupModelID: String
  let pausedDownload: PersistedModelDownload?
  let modelDownloadProgress: ModelDownloadProgress?
  let runtimeDetail: String

  static func make(runtimeBridge: RuntimeBridge) -> Self {
    let appSupportSetupDetail = AppSupportDirectories.prepareAppOwnedDirectories()
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
      activeModelInvalidationDetail: activeModelInvalidationDetail,
      appSupportSetupDetail: appSupportSetupDetail
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
    activeModelInvalidationDetail: String?,
    appSupportSetupDetail: String?
  ) -> String {
    var details = ["Amentia not started"]
    if pausedDownload != nil {
      details.append("paused model download available")
    }
    if let activeModelInvalidationDetail {
      details.append(activeModelInvalidationDetail)
    }
    if let appSupportSetupDetail {
      details.append(appSupportSetupDetail)
    }
    return details.joined(separator: ". ")
  }
}
