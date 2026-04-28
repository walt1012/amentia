import Foundation

struct LocalModelDownloadFinalizationPlan {
  let canActivateNow: Bool
  let preparedActivation: PreparedLocalModelActivation?

  var manifestPath: String? {
    preparedActivation?.manifestPath
  }
}

enum LocalModelDownloadFinalizer {
  static func prepare(
    model: LocalModelSummary,
    activationRequested: Bool,
    hasActiveOrPendingTurn: Bool
  ) throws -> LocalModelDownloadFinalizationPlan {
    try LocalModelActivationPreparer.validateDownloadedModel(model)

    let canActivateNow = !hasActiveOrPendingTurn
    guard activationRequested && canActivateNow else {
      return LocalModelDownloadFinalizationPlan(
        canActivateNow: canActivateNow,
        preparedActivation: nil
      )
    }

    return LocalModelDownloadFinalizationPlan(
      canActivateNow: canActivateNow,
      preparedActivation: PreparedLocalModelActivation(
        manifestPath: try LocalModelActivationPreparer.writeManifest(for: model)
      )
    )
  }
}
