import Foundation

@MainActor
extension AppViewModel {
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
    case .probeModel:
      probeLocalModel()
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
}
