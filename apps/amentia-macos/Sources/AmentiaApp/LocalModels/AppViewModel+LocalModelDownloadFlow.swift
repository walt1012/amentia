import Foundation

@MainActor
extension AppViewModel {
  func applyModelDownloadCancelPlan(_ cancelPlan: LocalModelDownloadCancelPlan) {
    switch cancelPlan.mode {
    case .running:
      runtimeDetail = cancelPlan.runtimeDetail
      modelDownloadCoordinator.cancelActiveDownload()
    case .orphanedPaused(let modelID):
      clearPausedModelDownload()
      removeIncompleteModelFile(modelID: modelID)
      modelDownloadState.clearProgress()
      runtimeDetail = cancelPlan.runtimeDetail
      refreshLocalModelCatalog()
    case .paused(let model):
      applyModelDownloadInterruptionPlan(
        LocalModelDownloadInterruptionPlanner.cancellationPlan(model: model),
        model: model
      )
      refreshLocalModelCatalog()
    }
  }

  func startModelDownloadTask(
    model: LocalModelSummary,
    downloadURL: URL,
    startPlan: LocalModelDownloadStartPlan,
    shouldActivateAfterDownload: Bool
  ) {
    let timelineThreadID = selectedThreadID
    let task = Task {
      defer {
        modelDownloadState.clearActiveDownload()
        modelDownloadCoordinator.finishActiveDownload()
        refreshLocalModelCatalog()
      }
      do {
        runtimeDetail = startPlan.runtimeDetail
        try await downloadModelFile(
          from: downloadURL,
          resumeData: startPlan.resumeData,
          modelID: model.id,
          expectedBytes: model.sizeBytes,
          to: URL(fileURLWithPath: model.installPath)
        )
        let completionState: LocalModelDownloadSessionCompletionState
        do {
          try await LocalModelDownloadSessionPlanner.validateDownloadedModelInBackground(
            model: model
          )
          completionState = try LocalModelDownloadSessionPlanner.completionStateAfterValidation(
            model: model,
            sourceURL: downloadURL,
            activationRequested: shouldActivateAfterDownload,
            hasActiveOrPendingTurn: hasActiveOrPendingTurn()
          )
        } catch LocalModelActivationPreparationError.integrityCheckFailed(let error) {
          removeIncompleteModelFile(modelID: model.id)
          throw LocalModelActivationPreparationError.integrityCheckFailed(error)
        }

        if let preparedActivation = completionState.preparedActivation {
          runtimeBridge.configureActiveLocalModel(
            manifestPath: preparedActivation.manifestPath,
            modelPath: model.installPath
          )
        }

        applyModelDownloadCompletionPlan(
          completionState.completionPlan,
          model: model,
          timelineThreadID: timelineThreadID
        )
      } catch {
        let interruptionPlan = LocalModelDownloadInterruptionPlanner.plan(model: model, error: error)
        applyModelDownloadInterruptionPlan(
          interruptionPlan,
          model: model,
          timelineThreadID: timelineThreadID
        )
      }
    }
    modelDownloadCoordinator.start(task)
  }

  func localModelDownloadRequestPlan(
    for model: LocalModelSummary
  ) -> LocalModelDownloadRequestPlan {
    localModelDownloadRequestPlanCache.plan(
      for: model,
      isDownloadRunning: modelDownloadCoordinator.isDownloading,
      pausedModelID: modelDownloadState.pausedModelID,
      resumeData: modelDownloadCoordinator.resumeData,
      currentProgress: modelDownloadState.progress
    )
  }

  func applyModelDownloadStartState(_ sessionState: LocalModelDownloadSessionStartState) {
    modelDownloadState.applyStart(sessionState)
    if sessionState.clearsPausedState {
      LocalModelDownloadStateStore.clearPausedDownload(coordinator: modelDownloadCoordinator)
    }
  }

  func applyModelDownloadCompletionPlan(
    _ plan: LocalModelDownloadCompletionPlan,
    model: LocalModelSummary,
    timelineThreadID: ThreadSummary.ID? = nil
  ) {
    switch plan.mode {
    case .activated, .waitingForTurn:
      selectedSetupModelID = model.id
    case .downloadedOnly:
      break
    }

    runtimeDetail = plan.runtimeDetail
    modelDownloadState.clearProgress()
    refreshLocalModelCatalog()
    appendEntry(
      to: timelineThreadID ?? selectedThreadID,
      TimelineEventPresenter.localModelDownloaded(plan)
    )

    if let relaunchRunningDetail = plan.relaunchRunningDetail,
       let relaunchIdleDetail = plan.relaunchIdleDetail
    {
      relaunchRuntimeIfNeeded(
        runningDetail: relaunchRunningDetail,
        idleDetail: relaunchIdleDetail
      )
    }
  }

  func applyModelDownloadInterruptionPlan(
    _ plan: LocalModelDownloadInterruptionPlan,
    model: LocalModelSummary,
    timelineThreadID: ThreadSummary.ID? = nil
  ) {
    switch plan.mode {
    case .paused(let resumeData):
      modelDownloadCoordinator.resumeData = resumeData
      modelDownloadState.markPaused(modelID: model.id)
      persistPausedModelDownload(modelID: model.id, resumeData: resumeData)
    case .cancelled, .failed:
      if plan.clearsPausedState {
        clearPausedModelDownload()
      }
      if plan.removesPartialFile {
        removeIncompleteModelFile(modelID: model.id)
      }
    }

    if plan.clearsProgress {
      modelDownloadState.clearProgress()
    }
    runtimeDetail = plan.runtimeDetail
    appendEntry(
      to: timelineThreadID ?? selectedThreadID,
      TimelineEventPresenter.localModelEvent(
        title: plan.timelineTitle,
        body: plan.timelineBody,
        model: model,
        kind: plan.timelineKind,
        attributes: plan.attributes
      )
    )
  }

  func downloadModelFile(
    from sourceURL: URL,
    resumeData: Data?,
    modelID: String,
    expectedBytes: Int64,
    to targetURL: URL
  ) async throws {
    let transfer = ModelDownloadTransfer(targetURL: targetURL) { [weak self] bytesReceived, totalBytes in
      Task { @MainActor [weak self] in
        self?.updateModelDownloadProgress(
          modelID: modelID,
          bytesReceived: bytesReceived,
          totalBytes: totalBytes > 0 ? totalBytes : expectedBytes
        )
      }
    }
    modelDownloadCoordinator.attachTransfer(transfer)
    try await transfer.start(from: sourceURL, resumeData: resumeData)
  }

  func updateModelDownloadProgress(
    modelID: String,
    bytesReceived: Int64,
    totalBytes: Int64
  ) {
    guard let progress = LocalModelDownloadProgressUpdater.updatedProgress(
      LocalModelDownloadProgressUpdate(
        modelID: modelID,
        activeModelID: modelDownloadState.activeModelID,
        currentProgress: modelDownloadState.progress,
        bytesReceived: bytesReceived,
        totalBytes: totalBytes,
        updatedAt: Date()
      )
    ) else {
      return
    }

    modelDownloadState.updateProgress(progress)
    runtimeDetail = modelDownloadProgressSummary()
  }

  func clearPausedModelDownload() {
    modelDownloadState.clearPausedDownload()
    LocalModelDownloadStateStore.clearPausedDownload(coordinator: modelDownloadCoordinator)
  }

  func persistPausedModelDownload(modelID: String, resumeData: Data) {
    LocalModelDownloadStateStore.persistPausedDownload(
      modelID: modelID,
      resumeData: resumeData,
      progress: modelDownloadState.progress
    )
  }

  func removeIncompleteModelFile(modelID: String) {
    LocalModelDownloadStateStore.removeIncompleteModelFile(modelID: modelID, models: localModels)
  }
}
