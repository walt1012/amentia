import Foundation

@MainActor
extension AppViewModel {
  func selectedLocalModelAction() -> LocalModelSelectedAction {
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

  func selectedModelActivationBlockedDetail() -> String {
    if hasActiveOrPendingTurn() {
      return "Finish or stop the current turn before switching models."
    }
    if runtimeState == .launching {
      return "Wait for the local engine to finish starting before switching models."
    }
    if modelDownloadCoordinator.isDownloading {
      return "Finish, pause, or cancel the current model download before switching models."
    }
    if modelDownloadState.hasPausedDownload {
      return "Continue or cancel the paused model download before switching models."
    }

    return "The selected local model is not ready to use."
  }
}
