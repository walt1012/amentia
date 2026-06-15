import Foundation

enum LocalExecutionSafetyModePresenter {
  static let defaultMode = "askBeforeChange"
  static let modes = ["explore", "askBeforeChange", "approvedWorkspaceExecution"]

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
      return "approved project execution"
    case "explore":
      return "explore"
    default:
      return value
    }
  }

  static func userTitle(_ value: String) -> String {
    switch value {
    case "explore":
      return "Explore"
    case "askBeforeChange":
      return "Ask Before Change"
    case "approvedWorkspaceExecution":
      return "Approved Project"
    default:
      return value
    }
  }

  static func userDetail(_ value: String) -> String {
    switch value {
    case "explore":
      return "Read-only exploration. Writes and shell commands are blocked."
    case "askBeforeChange":
      return "Default. Pith asks before file writes and shell commands."
    case "approvedWorkspaceExecution":
      return "Runs permitted project writes and shell commands without another approval."
    default:
      return "Custom local execution mode."
    }
  }

  static func validMode(_ value: String?) -> String {
    guard let value, modes.contains(value) else {
      return defaultMode
    }
    return value
  }
}
