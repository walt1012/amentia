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
  var models: [LocalModelSummary]
  var selectedSetupModelID: String

  init(
    modelHealth: ModelHealthSummary? = nil,
    runtimeReadiness: RuntimeReadinessSummary? = nil,
    models: [LocalModelSummary],
    selectedSetupModelID: String
  ) {
    self.modelHealth = modelHealth
    self.runtimeReadiness = runtimeReadiness
    self.models = models
    self.selectedSetupModelID = selectedSetupModelID
  }

  mutating func applyCatalogRefresh(_ refreshPlan: LocalModelCatalogRefreshPlan) {
    models = refreshPlan.models
    selectedSetupModelID = refreshPlan.selectedSetupModelID
  }

  mutating func clearRuntimeReadiness() {
    modelHealth = nil
    runtimeReadiness = nil
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
