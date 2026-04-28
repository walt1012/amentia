import Foundation

struct LocalModelActivationPlan {
  let timelineTitle: String
  let timelineBody: String
  let attributes: [String: String]
  let relaunchRunningDetail: String
  let relaunchIdleDetail: String
}

struct PreparedLocalModelActivation {
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
    LocalModelActivationPlan(
      timelineTitle: "Local Model Selected",
      timelineBody: "\(model.displayName) is now the active local model.",
      attributes: [
        "manifestPath": manifestPath,
        "modelId": model.id,
        "modelPath": model.installPath,
        "result": "selected",
      ],
      relaunchRunningDetail: "Restarting local runtime with \(model.displayName)...",
      relaunchIdleDetail: "\(model.displayName) will be used when the runtime launches."
    )
  }

  static func resetPlan() -> LocalModelActivationPlan {
    LocalModelActivationPlan(
      timelineTitle: "Local Model Reset",
      timelineBody: "Pith will use automatic local model discovery.",
      attributes: [
        "result": "reset"
      ],
      relaunchRunningDetail: "Restarting local runtime with automatic model discovery...",
      relaunchIdleDetail: "Automatic model discovery will be used when the runtime launches."
    )
  }

  static func selectionFailureDetail(error: Error) -> String {
    "Model selection failed: \(error.localizedDescription)"
  }
}
