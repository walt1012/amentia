import Foundation

enum LocalExecutionSafetyModePresenter {
  static func compact(_ value: String) -> String {
    switch value {
    case "askBeforeChange":
      return "Ask"
    case "approvedWorkspaceExecution":
      return "Approved"
    case "explore":
      return "Explore"
    default:
      return value
    }
  }

  static func detailed(_ value: String?) -> String {
    guard let value else {
      return "unknown"
    }

    switch value {
    case "askBeforeChange":
      return "ask-before-change"
    case "approvedWorkspaceExecution":
      return "approved workspace execution"
    case "explore":
      return "explore"
    default:
      return value
    }
  }
}
