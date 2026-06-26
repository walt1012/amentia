import Foundation

struct TimelineReceiptBadgeSummary: Hashable {
  let label: String
  let tone: StatusTone
}

enum TimelineReceiptBadgePresenter {
  static func badges(attributes: [String: String]) -> [TimelineReceiptBadgeSummary] {
    [
      actionReceiptBadge(attributes: attributes),
      sessionChangeBadge(attributes: attributes),
      webSearchBadge(attributes: attributes),
      connectorWorkflowBadge(attributes: attributes) ?? remoteWriteBadge(attributes: attributes),
    ]
      .compactMap { $0 }
  }

  private static func sessionChangeBadge(
    attributes: [String: String]
  ) -> TimelineReceiptBadgeSummary? {
    guard attributes["receiptKind"] == "sessionChangeRevert"
      || attributes["action"] == "thread.revertChanges"
    else {
      return nil
    }

    if attributes["revertedCount"] == "0" {
      return TimelineReceiptBadgeSummary(label: "No Files Reverted", tone: .neutral)
    }

    return TimelineReceiptBadgeSummary(label: "Files Reverted", tone: .ready)
  }

  private static func webSearchBadge(
    attributes: [String: String]
  ) -> TimelineReceiptBadgeSummary? {
    guard attributes["webSearchSourceMode"] == "searchResultAttribution" else {
      return nil
    }

    if attributes["pageFetchPerformed"] == "true" {
      return TimelineReceiptBadgeSummary(label: "Verified Sources", tone: .ready)
    }
    if attributes["sourceSnapshotAvailable"] == "true" {
      return TimelineReceiptBadgeSummary(label: "Search Snapshot", tone: .active)
    }

    return TimelineReceiptBadgeSummary(label: "Search Result Sources", tone: .active)
  }

  private static func actionReceiptBadge(
    attributes: [String: String]
  ) -> TimelineReceiptBadgeSummary? {
    guard attributes["actionReceiptSchema"] != nil
      || attributes["toolName"] != nil
      || attributes["tool"] != nil
    else {
      return nil
    }

    let tool = attributes["toolName"] ?? attributes["tool"]
    switch attributes["actionApprovalPolicy"] ?? inferredApprovalPolicy(tool) {
    case "autoApproved":
      return TimelineReceiptBadgeSummary(label: "Auto Approved", tone: .active)
    case "blocked":
      return TimelineReceiptBadgeSummary(label: "Blocked", tone: .warning)
    case "requiresApproval":
      return TimelineReceiptBadgeSummary(label: "Approval Required", tone: .warning)
    case "requiresPluginPermission":
      return TimelineReceiptBadgeSummary(label: "Plugin Permission", tone: .active)
    default:
      return TimelineReceiptBadgeSummary(label: "Ask Mode", tone: .ready)
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
  ) -> TimelineReceiptBadgeSummary? {
    guard let status = attributes["remoteWriteStatus"] else {
      return nil
    }

    switch status {
    case "completed":
      return TimelineReceiptBadgeSummary(label: "External Action Done", tone: .ready)
    case "notSent":
      return TimelineReceiptBadgeSummary(label: "External Action Not Sent", tone: .warning)
    case "unconfirmed":
      return TimelineReceiptBadgeSummary(label: "External Action Unconfirmed", tone: .warning)
    case "pending":
      return TimelineReceiptBadgeSummary(label: "External Action Pending", tone: .active)
    default:
      return TimelineReceiptBadgeSummary(label: "External Action Unknown", tone: .warning)
    }
  }

  private static func connectorWorkflowBadge(
    attributes: [String: String]
  ) -> TimelineReceiptBadgeSummary? {
    guard let status = attributes["connectorWorkflowStatus"] else {
      return nil
    }

    switch status {
    case "completed":
      return TimelineReceiptBadgeSummary(label: "Connection Done", tone: .ready)
    case "retryNeeded":
      return TimelineReceiptBadgeSummary(label: "Connection Retry Needed", tone: .warning)
    case "inspected":
      return TimelineReceiptBadgeSummary(label: "Connection Inspected", tone: .active)
    case "prepared":
      return TimelineReceiptBadgeSummary(label: "Connection Prepared", tone: .active)
    default:
      return TimelineReceiptBadgeSummary(label: "Connection Workflow", tone: .active)
    }
  }
}
