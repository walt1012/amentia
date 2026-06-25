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
      && !localDataResetInProgress
      && !hasActiveOrPendingTurn()
      && !isCheckingLocalModel
      && !localModelActivationCoordinator.isActivating
      && !pluginLifecycleOperations.isActive
  }

  func revealLocalDataFolder() {
    runtimeDetail = FileRevealService.revealFilePath(
      localDataFolderPath(),
      successDetail: "Revealed Amentia local data."
    )
  }

  func deleteLocalData() {
    guard canDeleteLocalData() else {
      runtimeDetail = LocalDataSettingsPresenter.deleteBlockedDetail
      return
    }

    localDataResetInProgress = true
    defer {
      localDataResetInProgress = false
    }
    runtimeBridge.stopRuntime(detail: "Deleting Amentia local data. Restart Amentia to continue.")
    runtimeLaunchCoordinator.cancel()
    workspaceOpenCoordinator.cancel()
    threadCreationCoordinator.cancel()
    threadHistoryLoadCoordinator.cancel()
    localExecutionRequests.clearAll()
    turnCancellationCoordinator.cancel()
    runtimeRelaunchCoordinator.cancel()
    localModelMetadataCoordinator.cancel()
    localModelProbeCoordinator.cancelPendingPostActivationCheck()
    localModelActivationCoordinator.cancel()
    pluginLifecycleOperations.cancel()
    modelDownloadCoordinator.cancelActiveDownload()
    modelDownloadCoordinator.finishActiveDownload()
    localModelDownloadRequestPlanCache.clear()

    do {
      let result = try AppDataResetService.deleteLocalData()
      applyLocalDataResetSuccess(result)
    } catch {
      runtimeDetail = UserFacingFailurePresenter.localDataDeletionFailureDetail()
      appendEntry(
        to: selectedThreadID,
        TimelineEntryFactory.warning(
          title: "Local Data Delete Failed",
          body: UserFacingFailurePresenter.localDataDeletionFailureDetail(),
          attributes: UserFacingFailurePresenter.technicalErrorAttributes(error)
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
      state.clearProbeState()
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
