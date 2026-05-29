import Foundation

struct TimelineLocalExecutionRecoverySummary: Hashable {
  let title: String
  let targetMode: String
  let detail: String
}

enum TimelineLocalExecutionRecoveryPresenter {
  static func recoveryAction(
    attributes: [String: String],
    currentMode: String
  ) -> TimelineLocalExecutionRecoverySummary? {
    guard attributes["actionApprovalPolicy"] == "blocked",
          attributes["blockReason"] == "readOnlyMode"
    else {
      return nil
    }

    let targetMode = "askBeforeChange"
    guard LocalExecutionSafetyModePresenter.validMode(currentMode) != targetMode else {
      return nil
    }

    let action = attributes["blockedAction"]
      ?? readableTool(attributes["toolName"] ?? attributes["tool"])
    return TimelineLocalExecutionRecoverySummary(
      title: "Switch to Ask Mode",
      targetMode: targetMode,
      detail: "Ask mode will let Pith request approval before it tries to \(action)."
    )
  }

  private static func readableTool(_ value: String?) -> String {
    switch value {
    case "run_shell":
      return "run a shell command"
    case "write_file":
      return "prepare a file write"
    default:
      return "make a local workspace change"
    }
  }
}
