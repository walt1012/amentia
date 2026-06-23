import Foundation

struct TimelineApprovalOutcomeSummary: Hashable {
  let title: String
  let detail: String
  let tone: StatusTone
}

enum TimelineApprovalOutcomePresenter {
  static func summary(attributes: [String: String]) -> TimelineApprovalOutcomeSummary? {
    guard let decision = attributes["decision"] else {
      return nil
    }

    let target = approvalTarget(attributes)
    switch decision {
    case "approved":
      return TimelineApprovalOutcomeSummary(
        title: "Approval accepted",
        detail: "Amentia is executing \(target). Review the receipt that follows.",
        tone: .ready
      )
    case "denied":
      return TimelineApprovalOutcomeSummary(
        title: "Approval denied",
        detail: "No local change was made. Adjust the request or ask Amentia for a safer version.",
        tone: .warning
      )
    default:
      return nil
    }
  }

  private static func approvalTarget(_ attributes: [String: String]) -> String {
    switch attributes["action"] {
    case "write_file":
      return readablePath(attributes["relativePath"], fallback: "the file change")
    case "run_shell":
      return "the approved shell command"
    case "run_plugin_command":
      return attributes["commandId"].map { "`\($0)`" } ?? "the approved plugin action"
    default:
      return attributes["action"] ?? "the approved action"
    }
  }

  private static func readablePath(_ value: String?, fallback: String) -> String {
    guard let value,
          !value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    else {
      return fallback
    }

    return "`\(value)`"
  }
}
