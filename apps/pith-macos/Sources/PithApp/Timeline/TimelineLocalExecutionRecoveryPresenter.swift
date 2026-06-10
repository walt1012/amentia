import Foundation

struct TimelineLocalExecutionRecoverySummary: Hashable {
  let title: String
  let targetMode: String
  let detail: String
  let retryMessage: String?
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
    let retryMessage = normalizedRetryMessage(attributes["retryMessage"])
    return TimelineLocalExecutionRecoverySummary(
      title: retryMessage == nil ? "Switch to Ask Mode" : "Switch Mode and Restore Request",
      targetMode: targetMode,
      detail: recoveryDetail(action: action, retryMessage: retryMessage),
      retryMessage: retryMessage
    )
  }

  private static func normalizedRetryMessage(_ value: String?) -> String? {
    let trimmed = value?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    return trimmed.isEmpty ? nil : trimmed
  }

  private static func recoveryDetail(action: String, retryMessage: String?) -> String {
    let base = "Ask mode will let Pith request approval before it tries to \(action)."
    guard retryMessage != nil else {
      return base
    }
    return "\(base) The original request will be restored to the composer."
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
