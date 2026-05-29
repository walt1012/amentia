import Foundation

struct TimelineContextReceiptSection: Identifiable, Equatable {
  let id: String
  let title: String
  let body: String
}

enum TimelineContextReceiptPresenter {
  static func sections(_ snapshot: TimelineInspectorSnapshot) -> [TimelineContextReceiptSection] {
    guard let entry = snapshot.selectedEntry else {
      return []
    }

    return [
      sourceSection(entry),
      actionSection(entry),
      memorySection(entry),
    ].compactMap { $0 }
  }

  static func sourceSummary(_ snapshot: TimelineInspectorSnapshot) -> String? {
    guard let entry = snapshot.selectedEntry else {
      return nil
    }

    return sourceSection(entry)?.body
  }

  static func actionSummary(_ snapshot: TimelineInspectorSnapshot) -> String? {
    guard let entry = snapshot.selectedEntry else {
      return nil
    }

    return actionSection(entry)?.body
  }

  static func memorySummary(_ snapshot: TimelineInspectorSnapshot) -> String? {
    guard let entry = snapshot.selectedEntry else {
      return nil
    }

    return memorySection(entry)?.body
  }

  private static func sourceSection(_ entry: TimelineEntry) -> TimelineContextReceiptSection? {
    let hasWebSource = entry.attributes["webSearchSourceMode"] != nil
      || entry.attributes["sourceAttribution"] == "web_search"
    guard hasWebSource else {
      return nil
    }

    let sourceMode = entry.attributes["webSearchSourceMode"] ?? "unknown"
    let pageFetch = entry.attributes["pageFetchPerformed"] ?? "unknown"
    let snapshotAvailable = entry.attributes["sourceSnapshotAvailable"] ?? "unknown"
    var lines = [
      "Source mode: \(sourceMode)",
      "Page fetch: \(yesNo(pageFetch))",
      "Source snapshot: \(yesNo(snapshotAvailable))",
    ]
    if let sourceTitles = entry.attributes["sourceTitles"] {
      lines.append("Titles: \(sourceTitles)")
    }
    if let sourceUrls = entry.attributes["sourceUrls"] {
      lines.append("URLs: \(sourceUrls)")
    }
    if let snapshotKind = entry.attributes["sourceSnapshotKind"] {
      lines.append("Snapshot kind: \(snapshotKind)")
    }
    if let snapshotHash = entry.attributes["sourceSnapshotHash"] {
      lines.append("Snapshot hash: \(snapshotHash)")
    }

    return TimelineContextReceiptSection(
      id: "source",
      title: "Web Search Sources",
      body: lines.joined(separator: "\n")
    )
  }

  private static func actionSection(_ entry: TimelineEntry) -> TimelineContextReceiptSection? {
    guard hasActionReceiptContext(entry.attributes) else {
      return nil
    }

    let tool = entry.attributes["toolName"] ?? entry.attributes["tool"]
    let mode = LocalExecutionSafetyModePresenter.detailed(
      entry.attributes["localExecutionSafetyMode"] ?? "askBeforeChange"
    )
    let boundary = readableBoundary(
      entry.attributes["actionBoundary"] ?? inferredBoundary(entry.attributes)
    )
    let policy = readableApprovalPolicy(
      entry.attributes["actionApprovalPolicy"] ?? inferredApprovalPolicy(tool)
    )
    let account = yesNo(entry.attributes["pithAccountRequired"] ?? "false")
    var lines = [
      "Mode: \(mode)",
      "Boundary: \(boundary)",
      "Approval: \(policy)",
      "Pith account required: \(account)",
    ]
    if let tool {
      lines.append("Tool: \(tool)")
    }
    if let workspace = entry.attributes["workspaceDisplayName"] {
      lines.append("Workspace: \(workspace)")
    }
    if let reason = entry.attributes["routingReason"] {
      lines.append("Reason: \(reason)")
    }

    return TimelineContextReceiptSection(
      id: "action",
      title: "Local Action",
      body: lines.joined(separator: "\n")
    )
  }

  private static func memorySection(_ entry: TimelineEntry) -> TimelineContextReceiptSection? {
    let noteCount = entry.attributes["memoryNoteCount"] ?? "0"
    let hasMemoryNotes = noteCount != "0"
    let hasMemoryContext = entry.attributes["memoryContextMode"] != nil
    if !hasMemoryNotes && !hasMemoryContext {
      return nil
    }

    var lines = ["Notes: \(noteCount)"]
    if hasMemoryNotes {
      let memoryTitles = entry.attributes["memoryNoteTitles"] ?? "Unavailable"
      let memoryIDs = entry.attributes["memoryNoteIds"] ?? "Unavailable"
      lines.append("Titles: \(memoryTitles)")
      lines.append("IDs: \(memoryIDs)")
    }

    if let memoryContextMode = entry.attributes["memoryContextMode"] {
      let estimatedChars = entry.attributes["memoryContextEstimatedChars"] ?? "unknown"
      let budgetChars = entry.attributes["memoryContextBudgetChars"] ?? "unknown"
      let omittedCount = entry.attributes["memoryContextOmittedNoteCount"] ?? "0"
      let truncatedCount = entry.attributes["memoryContextTruncatedNoteCount"] ?? "0"
      let candidateCount = entry.attributes["memoryContextCandidateNoteCount"] ?? noteCount
      let sourceCount = entry.attributes["memoryContextSourceNoteCount"] ?? candidateCount
      let windowTokens = entry.attributes["memoryContextWindowTokens"] ?? "unknown"
      lines.append(
        "Memory context: \(memoryContextMode) | \(noteCount)/\(candidateCount) relevant notes | "
          + "\(sourceCount) stored | \(estimatedChars)/\(budgetChars) chars | "
          + "\(windowTokens) token window | "
          + "omitted \(omittedCount) | truncated \(truncatedCount)"
      )
    }

    if let observationTruncated = entry.attributes["observationTruncated"] {
      let sourceChars = entry.attributes["observationSourceChars"] ?? "unknown"
      let budgetChars = entry.attributes["observationBudgetChars"] ?? "unknown"
      lines.append(
        "Observation: \(sourceChars)/\(budgetChars) chars | truncated \(observationTruncated)"
      )
    }

    return TimelineContextReceiptSection(
      id: "memory",
      title: "Memory Context",
      body: lines.joined(separator: "\n")
    )
  }

  private static func hasActionReceiptContext(_ attributes: [String: String]) -> Bool {
    attributes["actionReceiptSchema"] != nil
      || attributes["toolName"] != nil
      || attributes["tool"] != nil
      || attributes["agentToolName"] != nil
  }

  private static func readableBoundary(_ value: String?) -> String {
    switch value {
    case "workspace":
      return "workspace"
    case "network":
      return "network"
    case "localPlugin":
      return "local plugin"
    case "localRuntime":
      return "local runtime"
    default:
      return value ?? "unknown"
    }
  }

  private static func inferredBoundary(_ attributes: [String: String]) -> String {
    if attributes["webSearchSourceMode"] != nil || attributes["networkAccess"] == "true" {
      return "network"
    }
    switch attributes["toolKind"] ?? attributes["agentToolKind"] {
    case "web":
      return "network"
    case "connector", "plugin":
      return "localPlugin"
    case "file", "search", "shell", "workspace":
      return "workspace"
    default:
      return "localRuntime"
    }
  }

  private static func readableApprovalPolicy(_ value: String?) -> String {
    switch value {
    case "requiresApproval":
      return "requires approval"
    case "requiresPluginPermission":
      return "requires enabled plugin permission"
    case "readOnlyAllowed":
      return "read-only allowed"
    default:
      return value ?? "unknown"
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

  private static func yesNo(_ value: String) -> String {
    switch value {
    case "true":
      return "yes"
    case "false":
      return "no"
    default:
      return value
    }
  }
}
