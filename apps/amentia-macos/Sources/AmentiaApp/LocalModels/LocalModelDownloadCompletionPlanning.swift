import Foundation

enum LocalModelDownloadCompletionMode: Sendable {
  case downloadedOnly
  case activated
  case waitingForTurn
}

struct LocalModelDownloadCompletionPlan: Sendable {
  let mode: LocalModelDownloadCompletionMode
  let runtimeDetail: String
  let timelineBody: String
  let attributes: [String: String]
  let relaunchRunningDetail: String?
  let relaunchIdleDetail: String?
}

enum LocalModelDownloadCompletionPlanner {
  static func plan(
    model: LocalModelSummary,
    sourceURL: URL,
    activationRequested: Bool,
    canActivateNow: Bool,
    manifestPath: String?
  ) -> LocalModelDownloadCompletionPlan {
    let modelName = LocalModelDisplayPresenter.actionName(model)
    if activationRequested, canActivateNow, let manifestPath {
      return LocalModelDownloadCompletionPlan(
        mode: .activated,
        runtimeDetail: "Downloaded and selected \(modelName). Amentia will check it next.",
        timelineBody:
          "\(modelName) was downloaded and selected. Amentia will check it before cowork starts.",
        attributes: baseAttributes(model: model, sourceURL: sourceURL).merging(
          [
            "manifestPath": manifestPath,
            "result": "activated",
          ],
          uniquingKeysWith: { _, new in new }
        ),
        relaunchRunningDetail: "Restarting Amentia to check \(modelName)...",
        relaunchIdleDetail: "\(modelName) will be checked when Amentia starts."
      )
    }

    if activationRequested {
      return LocalModelDownloadCompletionPlan(
        mode: .waitingForTurn,
        runtimeDetail: "Downloaded \(modelName). Finish the current work before selecting it.",
        timelineBody:
          "\(modelName) was downloaded, but selection is waiting for current work to finish.",
        attributes: baseAttributes(model: model, sourceURL: sourceURL).merging(
          [
            "result": "downloaded_pending_activation",
          ],
          uniquingKeysWith: { _, new in new }
        ),
        relaunchRunningDetail: nil,
        relaunchIdleDetail: nil
      )
    }

    return LocalModelDownloadCompletionPlan(
      mode: .downloadedOnly,
      runtimeDetail: "Downloaded \(modelName). Use it when you are ready to switch models.",
      timelineBody: "\(modelName) was downloaded and can be selected later.",
      attributes: baseAttributes(model: model, sourceURL: sourceURL).merging(
        [
          "result": "downloaded",
        ],
        uniquingKeysWith: { _, new in new }
      ),
      relaunchRunningDetail: nil,
      relaunchIdleDetail: nil
    )
  }

  private static func baseAttributes(model: LocalModelSummary, sourceURL: URL) -> [String: String] {
    [
      "modelPath": model.installPath,
      "source": sourceURL.absoluteString,
    ]
  }
}

struct LocalModelDownloadFinalizationPlan: Sendable {
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
    hasActiveOrPendingTurn: Bool,
    validatesDownloadedModel: Bool = true
  ) throws -> LocalModelDownloadFinalizationPlan {
    if validatesDownloadedModel {
      try LocalModelActivationPreparer.validateDownloadedModel(model)
    }

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
