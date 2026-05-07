import Foundation

enum LocalModelDownloadCompletionMode {
  case downloadedOnly
  case activated
  case waitingForTurn
}

struct LocalModelDownloadCompletionPlan {
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
    if activationRequested, canActivateNow, let manifestPath {
      return LocalModelDownloadCompletionPlan(
        mode: .activated,
        runtimeDetail: "Downloaded and selected \(model.displayName).",
        timelineBody: "\(model.displayName) was downloaded and selected as the active local model.",
        attributes: baseAttributes(model: model, sourceURL: sourceURL).merging(
          [
            "manifestPath": manifestPath,
            "result": "activated",
          ],
          uniquingKeysWith: { _, new in new }
        ),
        relaunchRunningDetail: "Restarting local runtime with \(model.displayName)...",
        relaunchIdleDetail: "\(model.displayName) will be used when the runtime launches."
      )
    }

    if activationRequested {
      return LocalModelDownloadCompletionPlan(
        mode: .waitingForTurn,
        runtimeDetail: "Downloaded \(model.displayName). Finish the current turn before selecting it.",
        timelineBody:
          "\(model.displayName) was downloaded, but activation is waiting for the current local turn to finish.",
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
      runtimeDetail: "Downloaded \(model.displayName) to \(model.installPath).",
      timelineBody: "\(model.displayName) was downloaded to \(model.installPath).",
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
