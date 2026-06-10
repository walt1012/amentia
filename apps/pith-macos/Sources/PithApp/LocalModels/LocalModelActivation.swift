import Foundation

struct LocalModelActivationPlan: Sendable {
  let timelineTitle: String
  let timelineBody: String
  let attributes: [String: String]
  let relaunchRunningDetail: String
  let relaunchIdleDetail: String
}

struct LocalModelActivationFailurePlan: Sendable {
  let runtimeDetail: String
  let removesModelFile: Bool
  let refreshesCatalog: Bool
}

struct PreparedLocalModelActivation: Sendable {
  let manifestPath: String
}

enum LocalModelActivationPreparationError: LocalizedError {
  case integrityCheckFailed(Error)
  case manifestWriteFailed(Error)

  var errorDescription: String? {
    switch self {
    case .integrityCheckFailed(let error), .manifestWriteFailed(let error):
      return error.localizedDescription
    }
  }
}

enum LocalModelActivationPreparer {
  static func prepare(model: LocalModelSummary) throws -> PreparedLocalModelActivation {
    try validateDownloadedModel(model)
    return PreparedLocalModelActivation(manifestPath: try writeManifest(for: model))
  }

  static func prepareInBackground(
    model: LocalModelSummary
  ) async throws -> PreparedLocalModelActivation {
    try await withCheckedThrowingContinuation { continuation in
      DispatchQueue.global(qos: .utility).async {
        do {
          continuation.resume(returning: try prepare(model: model))
        } catch {
          continuation.resume(throwing: error)
        }
      }
    }
  }

  static func validateDownloadedModel(_ model: LocalModelSummary) throws {
    do {
      try LocalModelCatalog.validateDownloadedModel(model)
    } catch {
      throw LocalModelActivationPreparationError.integrityCheckFailed(error)
    }
  }

  static func writeManifest(for model: LocalModelSummary) throws -> String {
    do {
      return try LocalModelCatalog.writePackManifest(for: model)
    } catch {
      throw LocalModelActivationPreparationError.manifestWriteFailed(error)
    }
  }
}

enum LocalModelActivationPlanner {
  static func selectionPlan(model: LocalModelSummary, manifestPath: String) -> LocalModelActivationPlan {
    let modelName = LocalModelDisplayPresenter.actionName(model)
    LocalModelActivationPlan(
      timelineTitle: "Local Model Selected",
      timelineBody: "\(modelName) is now the active local model.",
      attributes: [
        "manifestPath": manifestPath,
        "modelId": model.id,
        "modelPath": model.installPath,
        "result": "selected",
      ],
      relaunchRunningDetail: "Restarting local service with \(modelName)...",
      relaunchIdleDetail: "\(modelName) will be used when the local service starts."
    )
  }

  static func resetPlan() -> LocalModelActivationPlan {
    LocalModelActivationPlan(
      timelineTitle: "Local Model Reset",
      timelineBody: "Pith will choose the local model automatically.",
      attributes: [
        "result": "reset"
      ],
      relaunchRunningDetail: "Restarting local service with automatic model discovery...",
      relaunchIdleDetail: "Automatic model discovery will be used when the local service starts."
    )
  }

  static func selectionFailureDetail(error: Error) -> String {
    "Model selection failed: \(error.localizedDescription)"
  }

  static func failurePlan(error: Error) -> LocalModelActivationFailurePlan {
    if let activationError = error as? LocalModelActivationPreparationError {
      switch activationError {
      case .integrityCheckFailed(let underlyingError):
        return LocalModelActivationFailurePlan(
          runtimeDetail: "Model integrity check failed: \(underlyingError.localizedDescription)",
          removesModelFile: true,
          refreshesCatalog: true
        )
      case .manifestWriteFailed(let underlyingError):
        return LocalModelActivationFailurePlan(
          runtimeDetail: selectionFailureDetail(error: underlyingError),
          removesModelFile: false,
          refreshesCatalog: false
        )
      }
    }

    return LocalModelActivationFailurePlan(
      runtimeDetail: selectionFailureDetail(error: error),
      removesModelFile: false,
      refreshesCatalog: false
    )
  }
}
