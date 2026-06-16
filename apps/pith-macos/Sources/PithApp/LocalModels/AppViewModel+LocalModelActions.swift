import Foundation

@MainActor
extension AppViewModel {
  func canDownloadRecommendedModel(modelID: String) -> Bool {
    guard let model = localModel(for: modelID),
          !model.downloaded,
          !localModelActivationCoordinator.isActivating
    else {
      return false
    }

    return localModelDownloadRequestPlan(for: model).canStart
  }

  func canActivateRecommendedModel(modelID: String) -> Bool {
    guard runtimeState != .launching,
          !hasActiveOrPendingTurn(),
          !localModelActivationCoordinator.isActivating,
          !modelDownloadCoordinator.isDownloading,
          !modelDownloadState.hasPausedDownload
    else {
      return false
    }
    guard let model = localModel(for: modelID) else {
      return false
    }

    return (model.downloaded || model.needsVerification) && !model.active
  }

  func canResetActiveLocalModel() -> Bool {
    runtimeState != .launching
      && !hasActiveOrPendingTurn()
      && !localModelActivationCoordinator.isActivating
      && !modelDownloadCoordinator.isDownloading
      && runtimeBridge.activeLocalModelPath() != nil
  }

  func canProbeLocalModel() -> Bool {
    runtimeState == .ready
      && isLocalModelReady()
      && !isCheckingLocalModel
      && !hasActiveOrPendingTurn()
      && !localModelActivationCoordinator.isActivating
      && !modelDownloadCoordinator.isDownloading
      && !modelDownloadState.hasPausedDownload
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
    guard !localModelActivationCoordinator.isActivating else {
      runtimeDetail = "Finish the current model selection check before downloading another model."
      return
    }

    guard let model = localModel(for: modelID) else {
      runtimeDetail = "The selected local model is unavailable."
      return
    }

    if model.needsVerification && !modelDownloadState.hasPausedDownload {
      removeIncompleteModelFile(modelID: model.id)
      refreshLocalModelCatalog()
      if let verifiedModel = localModel(for: model.id), verifiedModel.downloaded {
        activateRecommendedModel(modelID: verifiedModel.id)
        return
      }
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
    guard runtimeState != .launching else {
      runtimeDetail = "Wait for Pith to finish starting before switching models."
      return
    }
    guard !hasActiveOrPendingTurn() else {
      runtimeDetail = "Finish or stop the current work before switching models."
      return
    }
    guard !modelDownloadCoordinator.isDownloading,
          !modelDownloadState.hasPausedDownload
    else {
      runtimeDetail = "Finish the current model download before switching models."
      return
    }
    guard let requestID = localModelActivationCoordinator.begin() else {
      runtimeDetail = "Finish the current model selection check before switching models."
      return
    }

    guard let model = localModel(for: modelID) else {
      localModelActivationCoordinator.finish(requestID)
      runtimeDetail = "The selected local model is unavailable."
      return
    }

    guard model.downloaded || model.needsVerification else {
      localModelActivationCoordinator.finish(requestID)
      runtimeDetail = "Download \(LocalModelDisplayPresenter.actionName(model)) before using it."
      return
    }

    runtimeDetail = "Verifying \(LocalModelDisplayPresenter.actionName(model)) before selection..."
    let task = Task {
      defer {
        localModelActivationCoordinator.finish(requestID)
      }
      do {
        let preparedActivation = try await LocalModelActivationPreparer.prepareInBackground(
          model: model
        )
        guard localModelActivationCoordinator.isCurrent(requestID) else {
          return
        }
        guard !hasActiveOrPendingTurn() else {
          runtimeDetail = "Finish or stop the current work before switching models."
          return
        }
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
    localModelActivationCoordinator.bind(task: task, requestID: requestID)
  }

  func resetActiveLocalModel() {
    guard canResetActiveLocalModel() else {
      runtimeDetail =
        "Finish Pith startup, model download, model selection check, or active work before resetting model selection."
      return
    }

    runtimeBridge.clearActiveLocalModel()
    refreshLocalModelCatalog()
    applyLocalModelActivationPlan(LocalModelActivationPlanner.resetPlan())
  }

  func probeLocalModel() {
    guard canProbeLocalModel() else {
      runtimeDetail = "Finish startup, model download, model selection, or active work before checking the model."
      return
    }

    isCheckingLocalModel = true
    runtimeDetail = "Checking the active local model..."
    Task {
      defer {
        isCheckingLocalModel = false
      }

      do {
        let probe = try await runtimeBridge.probeModel()
        applyLocalModelProbe(probe)
      } catch {
        applyLocalModelProbeFailure(error)
      }
    }
  }

  private func applyLocalModelProbe(_ probe: RuntimeBridge.RuntimeModelProbe) {
    if probe.status == "ready" {
      runtimeDetail = "Local model check passed."
      var attributes = [
        "modelId": probe.modelID,
        "backend": probe.backend,
        "status": probe.status,
      ]
      if let sample = probe.sample?.trimmingCharacters(in: .whitespacesAndNewlines),
         !sample.isEmpty
      {
        attributes["sample"] = sample
      }
      appendEntry(
        to: selectedThreadID,
        TimelineEventPresenter.localModelProbe(
          title: "Local Model Checked",
          body: "The active local model answered a short local prompt.",
          attributes: attributes
        )
      )
      return
    }

    runtimeDetail = "Local model check failed. Re-download the model or restart Pith, then check again."
    appendEntry(
      to: selectedThreadID,
      TimelineEventPresenter.localModelProbe(
        title: "Local Model Check Failed",
        body: probe.detail,
        kind: .warning,
        attributes: [
          "modelId": probe.modelID,
          "backend": probe.backend,
          "status": probe.status,
        ]
      )
    )
  }

  private func applyLocalModelProbeFailure(_ error: Error) {
    runtimeDetail = "Local model check failed: \(error.localizedDescription)"
    appendEntry(
      to: selectedThreadID,
      TimelineEventPresenter.localModelProbe(
        title: "Local Model Check Failed",
        body: error.localizedDescription,
        kind: .warning,
        attributes: [
          "status": "request_failed"
        ]
      )
    )
  }
}
