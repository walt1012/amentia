import Foundation

struct LocalModelCatalogRefreshSnapshot {
  let storageRootPath: String
  let configuredActiveModelPath: String?
  let runtimeModelPath: String?
  let selectedSetupModelID: String
}

struct LocalModelCatalogRefreshPlan {
  let models: [LocalModelSummary]
  let selectedSetupModelID: String
  let shouldClearConfiguredActiveModel: Bool
}

struct LocalModelReadinessState {
  var modelHealth: ModelHealthSummary?
  var runtimeReadiness: RuntimeReadinessSummary?
  var probeState: LocalModelProbeState?
  var models: [LocalModelSummary]
  var selectedSetupModelID: String

  init(
    modelHealth: ModelHealthSummary? = nil,
    runtimeReadiness: RuntimeReadinessSummary? = nil,
    probeState: LocalModelProbeState? = nil,
    models: [LocalModelSummary],
    selectedSetupModelID: String
  ) {
    self.modelHealth = modelHealth
    self.runtimeReadiness = runtimeReadiness
    self.probeState = probeState
    self.models = models
    self.selectedSetupModelID = selectedSetupModelID
  }

  mutating func applyCatalogRefresh(_ refreshPlan: LocalModelCatalogRefreshPlan) {
    models = refreshPlan.models
    selectedSetupModelID = refreshPlan.selectedSetupModelID
    reconcileProbeStateWithActiveModel()
  }

  mutating func clearRuntimeReadiness() {
    modelHealth = nil
    runtimeReadiness = nil
    probeState = nil
  }

  mutating func clearProbeState() {
    probeState = nil
  }

  mutating func markProbeStarted(modelID: String) {
    probeState = LocalModelProbeState(modelID: modelID, status: .checking, detail: nil)
  }

  mutating func applyProbeResult(modelID: String, status: String, detail _: String?) {
    if status == "ready" {
      probeState = LocalModelProbeState(modelID: modelID, status: .passed, detail: nil)
      return
    }

    probeState = LocalModelProbeState(
      modelID: modelID,
      status: .failed,
      detail: LocalModelProbePresenter.readinessFailureDetail()
    )
  }

  func blocksReadiness(activeModelID: String?) -> Bool {
    guard let activeModelID,
          let probeState,
          probeState.modelID == activeModelID
    else {
      return false
    }

    return probeState.status.blocksReadiness
  }

  func probeFailureDetail(activeModelID: String?) -> String? {
    guard let activeModelID,
          let probeState,
          probeState.modelID == activeModelID,
          probeState.status == .failed
    else {
      return nil
    }

    return probeState.detail ?? "Local model startup failed. Restart Amentia to try starting it again."
  }

  private mutating func reconcileProbeStateWithActiveModel() {
    let activeModelID = models.first(where: { $0.active })?.id
    guard probeState?.modelID != activeModelID else {
      return
    }

    probeState = nil
  }
}

struct LocalModelProbeState: Equatable {
  let modelID: String
  let status: LocalModelProbeStatus
  let detail: String?
}

enum LocalModelProbeStatus: Equatable {
  case checking
  case passed
  case failed

  var blocksReadiness: Bool {
    switch self {
    case .checking, .failed:
      return true
    case .passed:
      return false
    }
  }
}

enum LocalModelCatalogRefreshPlanner {
  static func plan(_ snapshot: LocalModelCatalogRefreshSnapshot) -> LocalModelCatalogRefreshPlan {
    let activeModelPath = snapshot.configuredActiveModelPath ?? snapshot.runtimeModelPath
    var models = LocalModelCatalog.summaries(
      storageRootPath: snapshot.storageRootPath,
      activeModelPath: activeModelPath
    )
    let shouldClearConfiguredActiveModel = snapshot.configuredActiveModelPath != nil
      && !models.contains(where: { $0.active })

    if shouldClearConfiguredActiveModel {
      models = LocalModelCatalog.summaries(
        storageRootPath: snapshot.storageRootPath,
        activeModelPath: snapshot.runtimeModelPath
      )
    }

    let selectedSetupModelID = models.contains(where: { $0.id == snapshot.selectedSetupModelID })
      ? snapshot.selectedSetupModelID
      : LocalModelCatalog.defaultFirstUseModelID

    return LocalModelCatalogRefreshPlan(
      models: models,
      selectedSetupModelID: selectedSetupModelID,
      shouldClearConfiguredActiveModel: shouldClearConfiguredActiveModel
    )
  }
}
