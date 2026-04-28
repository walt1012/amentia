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
