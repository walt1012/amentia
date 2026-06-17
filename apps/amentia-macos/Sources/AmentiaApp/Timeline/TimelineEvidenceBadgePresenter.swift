import Foundation

struct TimelineEvidenceBadgeSummary: Hashable {
  let label: String
  let tone: StatusTone
}

enum TimelineEvidenceBadgePresenter {
  static func badges(attributes: [String: String]) -> [TimelineEvidenceBadgeSummary] {
    [
      actionReceiptBadge(attributes: attributes),
      webSearchBadge(attributes: attributes),
      connectorWorkflowBadge(attributes: attributes) ?? remoteWriteBadge(attributes: attributes),
    ]
      .compactMap { $0 }
  }

  private static func webSearchBadge(
    attributes: [String: String]
  ) -> TimelineEvidenceBadgeSummary? {
    guard attributes["webSearchSourceMode"] == "searchResultAttribution" else {
      return nil
    }

    if attributes["pageFetchPerformed"] == "true" {
      return TimelineEvidenceBadgeSummary(label: "Verified Sources", tone: .ready)
    }
    if attributes["sourceSnapshotAvailable"] == "true" {
      return TimelineEvidenceBadgeSummary(label: "Search Snapshot", tone: .active)
    }

    return TimelineEvidenceBadgeSummary(label: "Search Result Sources", tone: .active)
  }

  private static func actionReceiptBadge(
    attributes: [String: String]
  ) -> TimelineEvidenceBadgeSummary? {
    guard attributes["actionReceiptSchema"] != nil
      || attributes["toolName"] != nil
      || attributes["tool"] != nil
    else {
      return nil
    }

    let tool = attributes["toolName"] ?? attributes["tool"]
    switch attributes["actionApprovalPolicy"] ?? inferredApprovalPolicy(tool) {
    case "autoApproved":
      return TimelineEvidenceBadgeSummary(label: "Auto Approved", tone: .active)
    case "blocked":
      return TimelineEvidenceBadgeSummary(label: "Blocked", tone: .warning)
    case "requiresApproval":
      return TimelineEvidenceBadgeSummary(label: "Approval Required", tone: .warning)
    case "requiresPluginPermission":
      return TimelineEvidenceBadgeSummary(label: "Plugin Permission", tone: .active)
    default:
      return TimelineEvidenceBadgeSummary(label: "Ask Mode", tone: .ready)
    }
  }

  private static func inferredApprovalPolicy(_ tool: String?) -> String {
    switch tool {
    case "write_file", "run_shell":
      return "requiresApproval"
    case "web_search":
      return "requiresPluginPermission"
    default:
      return "readOnlyAllowed"
    }
  }

  private static func remoteWriteBadge(
    attributes: [String: String]
  ) -> TimelineEvidenceBadgeSummary? {
    guard let status = attributes["remoteWriteStatus"] else {
      return nil
    }

    switch status {
    case "completed":
      return TimelineEvidenceBadgeSummary(label: "External Action Done", tone: .ready)
    case "notSent":
      return TimelineEvidenceBadgeSummary(label: "External Action Not Sent", tone: .warning)
    case "unconfirmed":
      return TimelineEvidenceBadgeSummary(label: "External Action Unconfirmed", tone: .warning)
    case "pending":
      return TimelineEvidenceBadgeSummary(label: "External Action Pending", tone: .active)
    default:
      return TimelineEvidenceBadgeSummary(label: "External Action Unknown", tone: .warning)
    }
  }

  private static func connectorWorkflowBadge(
    attributes: [String: String]
  ) -> TimelineEvidenceBadgeSummary? {
    guard let status = attributes["connectorWorkflowStatus"] else {
      return nil
    }

    switch status {
    case "completed":
      return TimelineEvidenceBadgeSummary(label: "Connection Done", tone: .ready)
    case "retryNeeded":
      return TimelineEvidenceBadgeSummary(label: "Connection Retry Needed", tone: .warning)
    case "inspected":
      return TimelineEvidenceBadgeSummary(label: "Connection Inspected", tone: .active)
    case "prepared":
      return TimelineEvidenceBadgeSummary(label: "Connection Prepared", tone: .active)
    default:
      return TimelineEvidenceBadgeSummary(label: "Connection Workflow", tone: .active)
    }
  }
}
