import Foundation

enum RuntimeToolReadinessPresenter {
  static func hasToolChecks(_ checks: [RuntimeReadinessCheckSummary]) -> Bool {
    !toolChecks(checks).isEmpty
  }

  static func hasToolIssue(_ checks: [RuntimeReadinessCheckSummary]) -> Bool {
    primaryIssue(checks) != nil
  }

  static func timelineDetail(
    _ checks: [RuntimeReadinessCheckSummary],
    metrics: [String: String] = [:]
  ) -> String {
    guard let issue = primaryIssue(checks) else {
      return LocalExecutionSafetyPresenter.timelineDetail(metrics) ?? "Ready"
    }

    return "\(shortLabel(issue)) \(statusTitle(issue.status))"
  }

  static func timelineTone(_ checks: [RuntimeReadinessCheckSummary]) -> StatusTone {
    guard let issue = primaryIssue(checks) else {
      return .ready
    }

    return tone(issue.status)
  }

  static func inspectorSummary(
    _ checks: [RuntimeReadinessCheckSummary],
    metrics: [String: String] = [:]
  ) -> String? {
    guard let issue = primaryIssue(checks) else {
      return LocalExecutionSafetyPresenter.inspectorSummary(metrics)
    }

    return "\(longLabel(issue)) \(statusTitle(issue.status).lowercased())"
  }

  private static func primaryIssue(
    _ checks: [RuntimeReadinessCheckSummary]
  ) -> RuntimeReadinessCheckSummary? {
    toolChecks(checks).first(where: isBlockingIssue)
  }

  private static func isBlockingIssue(_ check: RuntimeReadinessCheckSummary) -> Bool {
    check.status != "ready" && check.status != "optional"
  }

  private static func toolChecks(
    _ checks: [RuntimeReadinessCheckSummary]
  ) -> [RuntimeReadinessCheckSummary] {
    ["executionControls", "webSearch", "nativeSandbox", "plugins"].compactMap { id in
      checks.first(where: { $0.id == id })
    }
  }

  private static func shortLabel(_ check: RuntimeReadinessCheckSummary) -> String {
    switch check.id {
    case "executionControls":
      return "Execution"
    case "webSearch":
      return "Web"
    case "nativeSandbox":
      return "Sandbox"
    case "plugins":
      return "Plugins"
    default:
      return check.title
    }
  }

  private static func longLabel(_ check: RuntimeReadinessCheckSummary) -> String {
    switch check.id {
    case "executionControls":
      return "Execution controls"
    case "webSearch":
      return "Web search"
    case "nativeSandbox":
      return "Native sandbox"
    case "plugins":
      return "Plugins"
    default:
      return check.title
    }
  }

  private static func statusTitle(_ status: String) -> String {
    switch status {
    case "limited":
      return "Limited"
    case "optional":
      return "Optional"
    case "setup_required":
      return "Setup"
    case "running":
      return "Running"
    case "needs_approval":
      return "Approval"
    case "failed":
      return "Failed"
    case "blocked":
      return "Blocked"
    default:
      return status.capitalized
    }
  }

  private static func tone(_ status: String) -> StatusTone {
    switch status {
    case "running":
      return .active
    case "limited", "optional", "setup_required", "needs_approval":
      return .warning
    default:
      return .danger
    }
  }
}

enum LocalExecutionSafetyPresenter {
  static func timelineDetail(_ metrics: [String: String]) -> String? {
    guard let mode = readableDefaultMode(metrics) else {
      return nil
    }
    return mode
  }

  static func inspectorSummary(_ metrics: [String: String]) -> String? {
    guard let mode = readableDefaultMode(metrics) else {
      return nil
    }
    let account = metrics["pithAccountRequired"] == "true"
      ? "account required"
      : "no account"
    return "Safety \(mode), \(account)"
  }

  private static func readableDefaultMode(_ metrics: [String: String]) -> String? {
    guard let rawMode = cleaned(metrics["defaultLocalExecutionSafetyMode"]) else {
      return nil
    }
    return LocalExecutionSafetyModePresenter.compact(rawMode)
  }

  private static func cleaned(_ value: String?) -> String? {
    guard let trimmed = value?.trimmingCharacters(in: .whitespacesAndNewlines),
          !trimmed.isEmpty
    else {
      return nil
    }
    return trimmed
  }
}
