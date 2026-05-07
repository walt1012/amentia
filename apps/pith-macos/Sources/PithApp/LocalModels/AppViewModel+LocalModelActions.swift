import Foundation

@MainActor
extension AppViewModel {
  func canDownloadRecommendedModel(modelID: String) -> Bool {
    guard let model = localModel(for: modelID),
          !model.downloaded
    else {
      return false
    }

    return localModelDownloadRequestPlan(for: model).canStart
  }

  func canActivateRecommendedModel(modelID: String) -> Bool {
    guard runtimeState != .launching,
          !hasActiveOrPendingTurn(),
          !modelDownloadCoordinator.isDownloading,
          !modelDownloadState.hasPausedDownload
    else {
      return false
    }
    guard let model = localModel(for: modelID) else {
      return false
    }

    return model.downloaded && !model.active
  }

  func canResetActiveLocalModel() -> Bool {
    runtimeState != .launching
      && !hasActiveOrPendingTurn()
      && !modelDownloadCoordinator.isDownloading
      && runtimeBridge.activeLocalModelPath() != nil
  }

  func canCancelModelDownload() -> Bool {
    modelDownloadCoordinator.isDownloading || modelDownloadState.hasPausedDownload
  }

  func canPauseModelDownload() -> Bool {
    modelDownloadCoordinator.canPause
  }

  func pauseModelDownload() {
    guard canPauseModelDownload() else {
      return
    }

    runtimeDetail = LocalModelDownloadControlPlanner.pauseDetail(
      activeModelID: modelDownloadState.activeModelID,
      models: localModels
    )
    modelDownloadCoordinator.pauseActiveTransfer()
  }

  func cancelModelDownload() {
    guard canCancelModelDownload(),
          let cancelPlan = LocalModelDownloadControlPlanner.cancelPlan(
            isDownloading: modelDownloadCoordinator.isDownloading,
            activeModelID: modelDownloadState.activeModelID,
            pausedModelID: modelDownloadState.pausedModelID,
            models: localModels
          )
    else {
      return
    }

    applyModelDownloadCancelPlan(cancelPlan)
  }

  func downloadRecommendedModel(modelID: String, activateAfterDownload: Bool = false) {
    guard let model = localModel(for: modelID) else {
      runtimeDetail = "The selected local model is unavailable."
      return
    }

    let requestPlan = localModelDownloadRequestPlan(for: model)
    guard let downloadURL = requestPlan.downloadURL else {
      runtimeDetail = requestPlan.blockedDetail ?? "The selected local model is not ready to download."
      return
    }

    let startPlan = LocalModelDownloadStartPlanner.plan(
      model: model,
      sourceURL: downloadURL,
      pausedModelID: modelDownloadState.pausedModelID,
      resumeData: modelDownloadCoordinator.resumeData,
      currentProgress: modelDownloadState.progress
    )
    let sessionState = LocalModelDownloadSessionPlanner.startState(
      model: model,
      startPlan: startPlan,
      activateAfterDownload: activateAfterDownload,
      isLocalModelReady: isLocalModelReady()
    )
    applyModelDownloadStartState(sessionState)
    appendEntry(
      to: selectedThreadID,
      TimelineEventPresenter.localModelEvent(
        title: startPlan.timelineTitle,
        body: startPlan.timelineBody,
        model: model,
        attributes: startPlan.attributes
      )
    )
    startModelDownloadTask(
      model: model,
      downloadURL: downloadURL,
      startPlan: startPlan,
      shouldActivateAfterDownload: sessionState.shouldActivateAfterDownload
    )
  }

  func activateRecommendedModel(modelID: String) {
    guard !hasActiveOrPendingTurn() else {
      runtimeDetail = "Finish or cancel the current local turn before switching models."
      return
    }

    guard let model = localModel(for: modelID) else {
      runtimeDetail = "The selected local model is unavailable."
      return
    }

    guard model.downloaded else {
      runtimeDetail = "Download \(model.displayName) before using it."
      return
    }

    do {
      let preparedActivation = try LocalModelActivationPreparer.prepare(model: model)
      runtimeBridge.configureActiveLocalModel(
        manifestPath: preparedActivation.manifestPath,
        modelPath: model.installPath
      )
      selectedSetupModelID = model.id
      refreshLocalModelCatalog()
      applyLocalModelActivationPlan(
        LocalModelActivationPlanner.selectionPlan(
          model: model,
          manifestPath: preparedActivation.manifestPath
        )
      )
    } catch {
      applyLocalModelActivationFailure(
        LocalModelActivationPlanner.failurePlan(error: error),
        model: model
      )
    }
  }

  func resetActiveLocalModel() {
    guard !hasActiveOrPendingTurn() else {
      runtimeDetail = "Finish or cancel the current local turn before resetting model selection."
      return
    }

    runtimeBridge.clearActiveLocalModel()
    refreshLocalModelCatalog()
    applyLocalModelActivationPlan(LocalModelActivationPlanner.resetPlan())
  }

  func revealRecommendedModel(modelID: String) {
    guard let model = localModel(for: modelID) else {
      runtimeDetail = "The selected local model is unavailable."
      return
    }

    runtimeDetail = FileRevealService.revealFilePath(
      model.installPath,
      successDetail: "Revealed \(model.displayName)."
    )
  }

  func revealSuggestedModelDirectory() {
    runtimeDetail = FileRevealService.revealSuggestedPath(
      metricKey: "suggestedModelPath",
      modelHealth: modelHealth,
      successDetail: "Opened the suggested local model folder."
    )
  }

  func canRevealSuggestedModelDirectory() -> Bool {
    FileRevealService.hasSuggestedPath(metricKey: "suggestedModelPath", modelHealth: modelHealth)
  }

  func revealSuggestedBinaryDirectory() {
    runtimeDetail = FileRevealService.revealSuggestedPath(
      metricKey: "suggestedBinaryPath",
      modelHealth: modelHealth,
      successDetail: "Opened the suggested llama.cpp binary folder."
    )
  }

  func canRevealSuggestedBinaryDirectory() -> Bool {
    FileRevealService.hasSuggestedPath(metricKey: "suggestedBinaryPath", modelHealth: modelHealth)
  }

  func canDownloadLocalModel() -> Bool {
    LocalModelSelectedActionPlanner.canRun(selectedLocalModelAction())
  }

  func downloadLocalModel() {
    switch selectedLocalModelAction() {
    case .activate(let modelID):
      activateRecommendedModel(modelID: modelID)
    case .download(let modelID):
      downloadRecommendedModel(modelID: modelID, activateAfterDownload: true)
    case .blocked(let detail):
      runtimeDetail = detail
    }
  }

  func canBootstrapModelPackMetadata() -> Bool {
    runtimeState == .ready && !modelDownloadCoordinator.isDownloading
  }

  func bootstrapModelPackMetadata() {
    guard canBootstrapModelPackMetadata() else {
      runtimeDetail = "Launch the runtime before preparing local model metadata."
      return
    }

    Task {
      do {
        let result = try await runtimeBridge.bootstrapModelPack()
        await refreshModelHealthState()
        let copiedSummary = result.copiedFiles.isEmpty
          ? "Pack metadata was already present."
          : "Prepared \(result.copiedFiles.count) local model metadata file(s)."
        runtimeDetail = "\(copiedSummary) Manifest: \(result.manifestPath)"
      } catch {
        runtimeDetail = "Model metadata bootstrap failed: \(error.localizedDescription)"
      }
    }
  }

  func runLocalModelPrimaryAction(_ action: LocalModelPrimaryAction?) {
    guard let action else {
      return
    }

    switch action {
    case .pauseDownload:
      pauseModelDownload()
    case .continueDownload(let modelID):
      downloadRecommendedModel(modelID: modelID, activateAfterDownload: !isLocalModelReady())
    case .downloadSelectedModel:
      downloadLocalModel()
    case .blockedDownload:
      break
    case .bootstrapModelPackMetadata:
      bootstrapModelPackMetadata()
    }
  }

  func selectedSetupModelDownloadBlockedDetail() -> String? {
    guard let model = selectedSetupModel(),
          !model.downloaded
    else {
      return nil
    }

    return localModelDownloadRequestPlan(for: model).blockedDetail
  }

  private func applyModelDownloadCancelPlan(_ cancelPlan: LocalModelDownloadCancelPlan) {
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

  private func startModelDownloadTask(
    model: LocalModelSummary,
    downloadURL: URL,
    startPlan: LocalModelDownloadStartPlan,
    shouldActivateAfterDownload: Bool
  ) {
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
          completionState = try LocalModelDownloadSessionPlanner.completionState(
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

        applyModelDownloadCompletionPlan(completionState.completionPlan, model: model)
      } catch {
        let interruptionPlan = LocalModelDownloadInterruptionPlanner.plan(model: model, error: error)
        applyModelDownloadInterruptionPlan(interruptionPlan, model: model)
      }
    }
    modelDownloadCoordinator.start(task)
  }

  private func selectedLocalModelAction() -> LocalModelSelectedAction {
    let model = selectedSetupModel()
    let requestPlan: LocalModelDownloadRequestPlan?
    if let model, !model.active, !model.downloaded {
      requestPlan = localModelDownloadRequestPlan(for: model)
    } else {
      requestPlan = nil
    }

    return LocalModelSelectedActionPlanner.action(
      LocalModelSelectedActionSnapshot(
        selectedModel: model,
        requestPlan: requestPlan,
        canActivateDownloadedModel: model.map { canActivateRecommendedModel(modelID: $0.id) } ?? false,
        activationBlockedDetail: selectedModelActivationBlockedDetail()
      )
    )
  }

  private func selectedModelActivationBlockedDetail() -> String {
    if hasActiveOrPendingTurn() {
      return "Finish or cancel the current local turn before switching models."
    }
    if runtimeState == .launching {
      return "Wait for the runtime to finish launching before switching models."
    }
    if modelDownloadCoordinator.isDownloading {
      return "Finish, pause, or cancel the current model download before switching models."
    }
    if modelDownloadState.hasPausedDownload {
      return "Continue or cancel the paused model download before switching models."
    }

    return "The selected local model is not ready to use."
  }

  private func localModelDownloadRequestPlan(
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

  private func applyModelDownloadStartState(_ sessionState: LocalModelDownloadSessionStartState) {
    modelDownloadState.applyStart(sessionState)
    if sessionState.clearsPausedState {
      LocalModelDownloadStateStore.clearPausedDownload(coordinator: modelDownloadCoordinator)
    }
  }

  private func applyModelDownloadCompletionPlan(
    _ plan: LocalModelDownloadCompletionPlan,
    model: LocalModelSummary
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
      to: selectedThreadID,
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

  private func applyModelDownloadInterruptionPlan(
    _ plan: LocalModelDownloadInterruptionPlan,
    model: LocalModelSummary
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
      to: selectedThreadID,
      TimelineEventPresenter.localModelEvent(
        title: plan.timelineTitle,
        body: plan.timelineBody,
        model: model,
        kind: plan.timelineKind,
        attributes: plan.attributes
      )
    )
  }

  private func applyLocalModelActivationPlan(_ plan: LocalModelActivationPlan) {
    appendEntry(
      to: selectedThreadID,
      TimelineEventPresenter.localModelActivated(plan)
    )
    relaunchRuntimeIfNeeded(
      runningDetail: plan.relaunchRunningDetail,
      idleDetail: plan.relaunchIdleDetail
    )
  }

  private func applyLocalModelActivationFailure(
    _ plan: LocalModelActivationFailurePlan,
    model: LocalModelSummary
  ) {
    if plan.removesModelFile {
      removeIncompleteModelFile(modelID: model.id)
    }
    if plan.refreshesCatalog {
      refreshLocalModelCatalog()
    }
    runtimeDetail = plan.runtimeDetail
  }

  private func relaunchRuntimeIfNeeded(runningDetail: String, idleDetail: String) {
    let plan = RuntimeRelaunchPlanner.plan(
      runtimeState: runtimeState,
      runningDetail: runningDetail,
      idleDetail: idleDetail
    )
    runtimeDetail = plan.runtimeDetail

    switch plan.action {
    case .stopAndLaunch:
      runtimeBridge.stopRuntime(detail: plan.stopDetail ?? runningDetail)
      launchRuntime(launchDetail: plan.launchDetail ?? runningDetail)
    case .stopAndLaunchAfterCurrentLaunchSettles:
      runtimeBridge.stopRuntime(detail: plan.stopDetail ?? runningDetail)
      Task {
        for _ in 0..<10 {
          if runtimeState != .launching {
            break
          }
          try? await Task.sleep(nanoseconds: 200_000_000)
        }
        if runtimeState == .launching {
          runtimeDetail = plan.launchTimeoutDetail ?? idleDetail
          return
        }
        launchRuntime(launchDetail: plan.launchDetail ?? runningDetail)
      }
    case .updateIdleDetail:
      break
    }
  }

  private func downloadModelFile(
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

  private func updateModelDownloadProgress(
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

  private func clearPausedModelDownload() {
    modelDownloadState.clearPausedDownload()
    LocalModelDownloadStateStore.clearPausedDownload(coordinator: modelDownloadCoordinator)
  }

  private func persistPausedModelDownload(modelID: String, resumeData: Data) {
    LocalModelDownloadStateStore.persistPausedDownload(
      modelID: modelID,
      resumeData: resumeData,
      progress: modelDownloadState.progress
    )
  }

  private func removeIncompleteModelFile(modelID: String) {
    LocalModelDownloadStateStore.removeIncompleteModelFile(modelID: modelID, models: localModels)
  }
}
