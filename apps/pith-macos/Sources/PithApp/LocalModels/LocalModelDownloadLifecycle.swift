import Foundation

struct LocalModelDownloadSessionStartState: Sendable {
  let activeModelID: String
  let pausedModelID: String?
  let progress: ModelDownloadProgress
  let clearsPausedState: Bool
  let shouldActivateAfterDownload: Bool
}

struct LocalModelDownloadSessionCompletionState: Sendable {
  let completionPlan: LocalModelDownloadCompletionPlan
  let preparedActivation: PreparedLocalModelActivation?
}

enum LocalModelDownloadSessionPlanner {
  static func startState(
    model: LocalModelSummary,
    startPlan: LocalModelDownloadStartPlan,
    activateAfterDownload: Bool,
    isLocalModelReady: Bool
  ) -> LocalModelDownloadSessionStartState {
    LocalModelDownloadSessionStartState(
      activeModelID: model.id,
      pausedModelID: nil,
      progress: startPlan.progress,
      clearsPausedState: true,
      shouldActivateAfterDownload: activateAfterDownload || !isLocalModelReady
    )
  }

  static func completionState(
    model: LocalModelSummary,
    sourceURL: URL,
    activationRequested: Bool,
    hasActiveOrPendingTurn: Bool
  ) throws -> LocalModelDownloadSessionCompletionState {
    try LocalModelActivationPreparer.validateDownloadedModel(model)

    return try completionStateAfterValidation(
      model: model,
      sourceURL: sourceURL,
      activationRequested: activationRequested,
      hasActiveOrPendingTurn: hasActiveOrPendingTurn
    )
  }

  static func completionStateAfterValidation(
    model: LocalModelSummary,
    sourceURL: URL,
    activationRequested: Bool,
    hasActiveOrPendingTurn: Bool
  ) throws -> LocalModelDownloadSessionCompletionState {
    let finalizationPlan = try LocalModelDownloadFinalizer.prepare(
      model: model,
      activationRequested: activationRequested,
      hasActiveOrPendingTurn: hasActiveOrPendingTurn,
      validatesDownloadedModel: false
    )
    let completionPlan = LocalModelDownloadCompletionPlanner.plan(
      model: model,
      sourceURL: sourceURL,
      activationRequested: activationRequested,
      canActivateNow: finalizationPlan.canActivateNow,
      manifestPath: finalizationPlan.manifestPath
    )

    return LocalModelDownloadSessionCompletionState(
      completionPlan: completionPlan,
      preparedActivation: finalizationPlan.preparedActivation
    )
  }

  static func validateDownloadedModelInBackground(model: LocalModelSummary) async throws {
    try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
      DispatchQueue.global(qos: .utility).async {
        do {
          try LocalModelActivationPreparer.validateDownloadedModel(model)
          continuation.resume(returning: ())
        } catch {
          continuation.resume(throwing: error)
        }
      }
    }
  }
}
