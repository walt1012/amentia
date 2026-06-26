import Foundation

final class LocalModelActivationCoordinator {
  private let taskSlot = CancellableTaskSlot()

  var isActivating: Bool {
    taskSlot.isActive
  }

  func begin() -> UUID? {
    taskSlot.begin()
  }

  func bind(task: Task<Void, Never>, requestID: UUID) {
    taskSlot.bind(task: task, requestID: requestID)
  }

  func isCurrent(_ requestID: UUID) -> Bool {
    taskSlot.isCurrent(requestID)
  }

  func finish(_ requestID: UUID) {
    taskSlot.finish(requestID)
  }

  func cancel() {
    taskSlot.cancel()
  }
}

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
    case .integrityCheckFailed:
      return "The selected model could not be verified. Download it again to repair setup."
    case .manifestWriteFailed:
      return "Amentia could not save the selected model setup. Restart Amentia, then try again."
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
    return LocalModelActivationPlan(
      timelineTitle: "Local Model Selected",
      timelineBody: "\(modelName) is selected. Amentia will start it before cowork begins.",
      attributes: [
        "manifestPath": manifestPath,
        "modelId": model.id,
        "modelPath": model.installPath,
        "result": "selected",
      ],
      relaunchRunningDetail: "Restarting Amentia with \(modelName)...",
      relaunchIdleDetail: "\(modelName) will start when Amentia opens."
    )
  }

  static func resetPlan() -> LocalModelActivationPlan {
    LocalModelActivationPlan(
      timelineTitle: "Local Model Reset",
      timelineBody: "Amentia will choose the local model automatically.",
      attributes: [
        "result": "reset"
      ],
      relaunchRunningDetail: "Restarting Amentia with automatic model discovery...",
      relaunchIdleDetail: "Automatic model discovery will be used when Amentia starts."
    )
  }

  static func selectionFailureDetail(error: Error) -> String {
    if let activationError = error as? LocalModelActivationPreparationError {
      return activationError.localizedDescription
    }

    return "Model selection failed. Restart Amentia, then try selecting the model again."
  }

  static func failurePlan(error: Error) -> LocalModelActivationFailurePlan {
    if let activationError = error as? LocalModelActivationPreparationError {
      switch activationError {
      case .integrityCheckFailed:
        return LocalModelActivationFailurePlan(
          runtimeDetail:
            "The selected model could not be verified. Amentia removed the bad file. Download the model again to repair setup.",
          removesModelFile: true,
          refreshesCatalog: true
        )
      case .manifestWriteFailed:
        return LocalModelActivationFailurePlan(
          runtimeDetail: selectionFailureDetail(error: activationError),
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
