import Foundation

@MainActor
extension AppViewModel {
  func shouldShowSetupModelChoice() -> Bool {
    let actionSnapshot = localModelActionSnapshot()
    return runtimeState == .ready
      && !isLocalModelReady()
      && !actionSnapshot.hasModelDownload
      && actionSnapshot.pausedModelDownloadID == nil
      && !localModels.isEmpty
  }

  func canChangeSetupModelChoice() -> Bool {
    shouldShowSetupModelChoice()
  }

  func setupModelChoiceDetail() -> String {
    LocalModelOperationPresenter.setupModelChoiceDetail(
      localModelOperationSnapshot(),
      defaultModelID: LocalModelCatalog.defaultFirstUseModelID
    )
  }

  func setupDefaultModelID() -> String {
    LocalModelCatalog.defaultFirstUseModelID
  }

  func modelSetupCalloutTitle() -> String {
    localModelSetupGuidance().title
  }

  func modelSetupCalloutSummary() -> String {
    localModelSetupGuidance().summary
  }

  func modelSetupCalloutDetail() -> String {
    if shouldShowModelDownloadProgress() {
      return modelDownloadProgressSummary()
    }
    if let blockedDetail = selectedSetupModelDownloadBlockedDetail() {
      return blockedDetail
    }

    return localModelSetupGuidance().detail
  }

  func modelSetupCalloutTone() -> StatusTone {
    localModelSetupGuidance().tone
  }

  func modelSetupCalloutActionTitle() -> String? {
    let snapshot = localModelActionSnapshot()
    return LocalModelActionPlanner.primaryTitle(
      for: LocalModelActionPlanner.setupPrimaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func canRunModelSetupCalloutAction() -> Bool {
    let snapshot = localModelActionSnapshot()
    return LocalModelActionPlanner.canRun(
      LocalModelActionPlanner.setupPrimaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func runModelSetupCalloutAction() {
    let snapshot = localModelActionSnapshot()
    let action = LocalModelActionPlanner.setupPrimaryAction(snapshot)
    guard LocalModelActionPlanner.canRun(action, snapshot: snapshot) else {
      return
    }

    runLocalModelPrimaryAction(action)
  }

  func modelSetupCalloutSecondaryActionTitle() -> String? {
    LocalModelActionPlanner.secondaryTitle(
      for: LocalModelActionPlanner.setupSecondaryAction(localModelActionSnapshot())
    )
  }

  func canRunModelSetupCalloutSecondaryAction() -> Bool {
    let snapshot = localModelActionSnapshot()
    return LocalModelActionPlanner.canRun(
      LocalModelActionPlanner.setupSecondaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func runModelSetupCalloutSecondaryAction() {
    guard canRunModelSetupCalloutSecondaryAction() else {
      return
    }

    cancelModelDownload()
  }

  func modelDisplayName() -> String {
    LocalModelStatusPresenter.displayName(localModelStatusSnapshot())
  }

  func modelStatusSummary() -> String {
    LocalModelStatusPresenter.statusSummary(localModelStatusSnapshot())
  }

  func modelActionSummary() -> String {
    localModelSetupGuidance().actionSummary
  }

  func showsModelActivity() -> Bool {
    LocalModelStatusPresenter.showsActivity(localModelStatusSnapshot())
  }

  func isModelActionBlocking() -> Bool {
    LocalModelOperationPresenter.isActionBlocking(localModelOperationSnapshot())
  }

  func localModelPrimaryActionTitle() -> String? {
    let snapshot = localModelActionSnapshot()
    return LocalModelActionPlanner.primaryTitle(
      for: LocalModelActionPlanner.managerPrimaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func canRunLocalModelPrimaryAction() -> Bool {
    let snapshot = localModelActionSnapshot()
    return LocalModelActionPlanner.canRun(
      LocalModelActionPlanner.managerPrimaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func runLocalModelPrimaryAction() {
    let snapshot = localModelActionSnapshot()
    let action = LocalModelActionPlanner.managerPrimaryAction(snapshot)
    guard LocalModelActionPlanner.canRun(action, snapshot: snapshot) else {
      return
    }

    runLocalModelPrimaryAction(action)
  }

  func localModelSecondaryActionTitle() -> String? {
    LocalModelActionPlanner.secondaryTitle(
      for: LocalModelActionPlanner.managerSecondaryAction(localModelActionSnapshot())
    )
  }

  func canRunLocalModelSecondaryAction() -> Bool {
    let snapshot = localModelActionSnapshot()
    return LocalModelActionPlanner.canRun(
      LocalModelActionPlanner.managerSecondaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func runLocalModelSecondaryAction() {
    guard canRunLocalModelSecondaryAction() else {
      return
    }

    cancelModelDownload()
  }

  func modelDetailSummary() -> String {
    LocalModelStatusPresenter.detailSummary(localModelStatusSnapshot())
  }

  func modelSourceSummary() -> String {
    LocalModelStatusPresenter.sourceSummary(localModelStatusSnapshot())
  }

  func modelMetricsSummary() -> String {
    LocalModelStatusPresenter.metricsSummary(localModelStatusSnapshot())
  }

  func modelReadinessSummary() -> String {
    LocalModelStatusPresenter.readinessSummary(localModelStatusSnapshot())
  }

  func modelInstallHintSummary() -> String {
    LocalModelStatusPresenter.installHintSummary(localModelStatusSnapshot())
  }

  func modelSuggestedPathSummary() -> String {
    LocalModelStatusPresenter.suggestedPathSummary(localModelStatusSnapshot())
  }

  func modelArtifactPathSummary() -> String {
    LocalModelStatusPresenter.artifactPathSummary(localModelStatusSnapshot())
  }

  func modelManagerSummary() -> String {
    LocalModelOperationPresenter.managerSummary(localModelOperationSnapshot())
  }

  func localModelManagerRuleSummary() -> String {
    LocalModelStatusPresenter.managerRuleSummary(localModelStatusSnapshot())
  }

  func shouldShowModelDownloadProgress() -> Bool {
    LocalModelStatusPresenter.shouldShowDownloadProgress(localModelStatusSnapshot())
  }

  func modelDownloadProgressValue() -> Double? {
    LocalModelStatusPresenter.downloadProgressValue(localModelStatusSnapshot())
  }

  func modelDownloadProgressSummary() -> String {
    LocalModelStatusPresenter.downloadProgressSummary(localModelStatusSnapshot())
  }

  func localModelStatusSummary(_ model: LocalModelSummary) -> String {
    LocalModelStatusPresenter.localModelStatusSummary(model, snapshot: localModelStatusSnapshot())
  }

  func localModelChoiceSummary(_ model: LocalModelSummary) -> String {
    LocalModelStatusPresenter.localModelChoiceSummary(
      model,
      snapshot: localModelStatusSnapshot(),
      defaultModelID: LocalModelCatalog.defaultFirstUseModelID
    )
  }

  func defaultModelDownloadButtonTitle() -> String {
    LocalModelStatusPresenter.defaultDownloadButtonTitle(localModelStatusSnapshot())
  }

  func localModelDownloadButtonTitle(_ model: LocalModelSummary) -> String {
    LocalModelStatusPresenter.downloadButtonTitle(model, snapshot: localModelStatusSnapshot())
  }

  func localModelTagSummary(_ model: LocalModelSummary) -> String {
    LocalModelStatusPresenter.tagSummary(model)
  }

  func localModelPathSummary(_ model: LocalModelSummary) -> String {
    LocalModelStatusPresenter.pathSummary(model)
  }
}
