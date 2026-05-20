import Foundation

@MainActor
extension AppViewModel {
  func isLocalModelReady() -> Bool {
    guard runtimeState == .ready,
          let modelHealth,
          modelHealth.status == "ready",
          hasActiveCatalogModel()
    else {
      return false
    }

    return (modelHealth.metrics["readiness"] ?? "unknown") == "ready"
  }

  func refreshLocalModelCatalog() {
    let configuredActiveModelPath = runtimeBridge.activeLocalModelPath()
    let activeModelInvalidationDetail = runtimeBridge.consumeActiveLocalModelInvalidationDetail()
    let refreshPlan = LocalModelCatalogRefreshPlanner.plan(
      LocalModelCatalogRefreshSnapshot(
        storageRootPath: runtimeBridge.localModelStorageRootPath(),
        configuredActiveModelPath: configuredActiveModelPath,
        runtimeModelPath: modelHealth?.modelPath,
        selectedSetupModelID: selectedSetupModelID
      )
    )
    if refreshPlan.shouldClearConfiguredActiveModel {
      runtimeBridge.clearActiveLocalModel()
    }
    updateLocalModelReadinessState { state in
      state.applyCatalogRefresh(refreshPlan)
    }
    if let activeModelInvalidationDetail {
      runtimeDetail = activeModelInvalidationDetail
    }
    AppPreferences.storeSelectedSetupModelID(refreshPlan.selectedSetupModelID)
  }

  func localModelStatusSnapshot() -> LocalModelStatusSnapshot {
    return LocalModelStatusSnapshot(
      runtimeState: runtimeState,
      modelHealth: modelHealth,
      modelDownloadID: modelDownloadState.activeModelID,
      pausedModelDownloadID: modelDownloadState.pausedModelID,
      modelDownloadProgress: modelDownloadState.progress,
      selectedSetupModelID: selectedSetupModelID,
      selectedSetupModel: selectedSetupModel(),
      hasActiveCatalogModel: hasActiveCatalogModel()
    )
  }

  func selectedSetupModel() -> LocalModelSummary? {
    localModel(for: selectedSetupModelID)
      ?? localModel(for: LocalModelCatalog.defaultFirstUseModelID)
      ?? localModels.first
  }

  func localModel(for modelID: String?) -> LocalModelSummary? {
    guard let modelID else {
      return nil
    }

    return localModels.first(where: { $0.id == modelID })
  }

  func localModelSetupGuidance() -> LocalModelSetupGuidance {
    LocalModelOperationPresenter.setupGuidance(localModelOperationSnapshot())
  }

  func localModelOperationSnapshot() -> LocalModelOperationSnapshot {
    let downloadedModels = localModels.filter { $0.downloaded }
    let downloadedLocalSize = downloadedModels
      .compactMap { $0.localSizeBytes }
      .reduce(Int64(0), +)

    return LocalModelOperationSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady(),
      hasActiveTurn: hasActiveOrPendingTurn(),
      downloadingModel: localModel(for: modelDownloadState.activeModelID),
      pausedModel: localModel(for: modelDownloadState.pausedModelID),
      selectedSetupModel: selectedSetupModel(),
      selectedDownloadBlockedDetail: selectedSetupModelDownloadBlockedDetail(),
      downloadedModelCount: downloadedModels.count,
      totalModelCount: localModels.count,
      activeModelDisplayName: activeLocalModel()?.displayName,
      downloadedLocalSizeBytes: downloadedLocalSize
    )
  }

  func localModelActionSnapshot() -> LocalModelActionSnapshot {
    let canDownloadPausedModel = modelDownloadState.pausedModelID
      .map { canDownloadRecommendedModel(modelID: $0) }
      ?? false

    return LocalModelActionSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady(),
      hasModelDownload: modelDownloadState.hasActiveDownload,
      pausedModelDownloadID: modelDownloadState.pausedModelID,
      selectedDownloadBlockedDetail: selectedSetupModelDownloadBlockedDetail(),
      canPauseDownload: canPauseModelDownload(),
      canDownloadPausedModel: canDownloadPausedModel,
      canDownloadSelectedModel: canDownloadLocalModel(),
      canBootstrapModelPackMetadata: canBootstrapModelPackMetadata(),
      canCancelDownload: canCancelModelDownload(),
      defaultDownloadTitle: defaultModelDownloadButtonTitle()
    )
  }

  private func activeLocalModel() -> LocalModelSummary? {
    localModels.first(where: { $0.active })
  }

  private func hasActiveCatalogModel() -> Bool {
    localModels.contains(where: { $0.active })
  }
}
