import Foundation

struct LocalModelActivationPlan {
  let timelineTitle: String
  let timelineBody: String
  let attributes: [String: String]
  let relaunchRunningDetail: String
  let relaunchIdleDetail: String
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
