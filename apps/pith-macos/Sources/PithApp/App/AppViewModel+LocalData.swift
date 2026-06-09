import Foundation

@MainActor
extension AppViewModel {
  func localDataFolderPath() -> String {
    AppSupportDirectories.rootDirectory().path
  }

  func localDataStorageSummary() -> String {
    let downloadedBytes = localModelOperationSnapshot().downloadedLocalSizeBytes
    if downloadedBytes > 0 {
      return "Downloaded models use \(LocalModelByteFormatter.string(downloadedBytes)) on this Mac."
    }

    return "No downloaded model files yet. Sessions, plugins, and preferences stay local."
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
      runtimeDetail =
        "Finish active local work, model downloads, model selection checks, or plugin operations before deleting local data."
      return
    }

    runtimeBridge.stopRuntime(detail: "Local data reset. Relaunch the local engine to continue.")
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
    runtimeDetail = "Deleted Pith local data at \(result.appSupportPath). Relaunch the local engine to set up again."
    appendEntry(
      to: selectedThreadID,
      TimelineEntryFactory.system(
        title: "Local Data Deleted",
        body:
          "Pith removed local models, sessions, plugins, download recovery data, and known preferences. Workspaces on disk were not deleted.",
        attributes: [
          "appSupportPath": result.appSupportPath,
          "recreatedDirectoryCount": "\(result.recreatedDirectoryCount)",
        ]
      )
    )
  }
}
