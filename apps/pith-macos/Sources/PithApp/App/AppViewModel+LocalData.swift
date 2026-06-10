import Foundation

@MainActor
extension AppViewModel {
  func localDataFolderPath() -> String {
    AppSupportDirectories.rootDirectory().path
  }

  func localDataSettingsSummary() -> LocalDataSettingsSummary {
    LocalDataSettingsPresenter.summary(
      LocalDataSettingsSnapshot(
        downloadedModelBytes: localModelOperationSnapshot().downloadedLocalSizeBytes,
        canDeleteLocalData: canDeleteLocalData(),
        localDataPath: localDataFolderPath()
      )
    )
  }

  func canDeleteLocalData() -> Bool {
    runtimeState != .launching
      && !hasActiveOrPendingTurn()
      && !modelDownloadCoordinator.isDownloading
      && !localModelActivationCoordinator.isActivating
      && !pluginLifecycleOperations.isActive
  }

  func revealLocalDataFolder() {
    runtimeDetail = FileRevealService.revealFilePath(
      localDataFolderPath(),
      successDetail: "Revealed Pith local data."
    )
  }

  func deleteLocalData() {
    guard canDeleteLocalData() else {
      runtimeDetail = LocalDataSettingsPresenter.deleteBlockedDetail
      return
    }

    runtimeBridge.stopRuntime(detail: "Local data reset. Restart the local service to continue.")
    runtimeLaunchCoordinator.cancel()
    workspaceOpenCoordinator.cancel()
    threadCreationCoordinator.cancel()
    threadHistoryLoadCoordinator.cancel()
    localExecutionRequests.clearAll()
    turnCancellationCoordinator.cancel()
    runtimeRelaunchCoordinator.cancel()
    localModelMetadataCoordinator.cancel()
    localModelActivationCoordinator.cancel()
    pluginLifecycleOperations.cancel()
    modelDownloadCoordinator.cancelActiveDownload()
    modelDownloadCoordinator.finishActiveDownload()
    localModelDownloadRequestPlanCache.clear()

    do {
      let result = try AppDataResetService.deleteLocalData()
      applyLocalDataResetSuccess(result)
    } catch {
      runtimeDetail = "Local data reset failed: \(error.localizedDescription)"
      appendEntry(
        to: selectedThreadID,
        TimelineEntryFactory.warning(
          title: "Local Data Reset Failed",
          body: error.localizedDescription
        )
      )
    }
  }

  private func applyLocalDataResetSuccess(_ result: AppDataResetResult) {
    selectedLocalExecutionSafetyMode = AppPreferences.storedLocalExecutionSafetyMode()
    workspace = nil
    resetWorkspaceSearch()
    updateLocalModelReadinessState { state in
      state.models = LocalModelCatalog.summaries(
        storageRootPath: runtimeBridge.localModelStorageRootPath(),
        activeModelPath: nil
      )
      state.selectedSetupModelID = LocalModelCatalog.defaultFirstUseModelID
      state.modelHealth = nil
      state.runtimeReadiness = nil
    }
    updateMemoryState { state in
      state.resetRuntimeData()
      state.clearDraft()
    }
    updatePluginState { state in
      state.reset()
    }
    modelDownloadState = LocalModelDownloadRuntimeState(
      activeModelID: nil,
      pausedModelID: nil,
      progress: nil
    )
    resetToWelcomeThread()
    let resetSummary = LocalDataSettingsPresenter.resetSummary(result)
    runtimeDetail = resetSummary.runtimeDetail
    appendEntry(
      to: selectedThreadID,
      TimelineEntryFactory.system(
        title: resetSummary.timelineTitle,
        body: resetSummary.timelineBody,
        attributes: resetSummary.attributes
      )
    )
  }
}
