import Foundation

struct TimelineContextReceiptSection: Identifiable, Equatable {
  let id: String
  let title: String
  let body: String
}

enum TimelineContextReceiptPresenter {
  static func cardSummary(_ entry: TimelineEntry) -> String? {
    let parts = [
      actionCardSummary(entry.attributes),
      sourceCardSummary(entry.attributes),
      memoryCardSummary(entry.attributes),
      compactionCardSummary(entry.attributes),
    ].compactMap { $0 }

    guard !parts.isEmpty else {
      return nil
    }

    return parts.joined(separator: " | ")
  }

  static func sections(_ snapshot: TimelineInspectorSnapshot) -> [TimelineContextReceiptSection] {
    guard let entry = snapshot.selectedEntry else {
      return []
    }

    return [
      workspaceSection(entry),
      sourceSection(entry),
      actionSection(entry),
      memorySection(entry),
      compactionSection(entry),
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

  private static func workspaceSection(_ entry: TimelineEntry) -> TimelineContextReceiptSection? {
    guard hasWorkspaceContext(entry.attributes) else {
      return nil
    }

    var lines: [String] = []
    appendLine("Tool", entry.attributes["tool"] ?? entry.attributes["agentToolName"], to: &lines)
    appendLine("Workspace", entry.attributes["workspaceDisplayName"], to: &lines)
    appendLine("Path", entry.attributes["relativePath"], to: &lines)
    appendLine("Query", entry.attributes["query"], to: &lines)
    appendLine("Max bytes", entry.attributes["maxBytes"], to: &lines)
    appendLine("Max results", entry.attributes["maxResults"], to: &lines)
    appendLine("Result count", entry.attributes["resultCount"], to: &lines)
    appendLine("Unique paths", entry.attributes["uniquePathCount"], to: &lines)
    appendLine("Truncated", entry.attributes["isTruncated"].map(yesNo), to: &lines)
    appendLine("Next action", entry.attributes["nextAction"], to: &lines)
    appendLine("Next path", entry.attributes["nextRelativePath"], to: &lines)
    appendLine("Loop step", entry.attributes["agentStepIndex"], to: &lines)

    guard !lines.isEmpty else {
      return nil
    }

    return TimelineContextReceiptSection(
      id: "workspace",
      title: "Workspace Context",
      body: lines.joined(separator: "\n")
    )
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
    if let attribution = entry.attributes["sourceAttribution"] {
      lines.append("Attribution: \(attribution)")
    }
    if let reason = entry.attributes["routingReason"] {
      lines.append("Reason: \(reason)")
    }
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
    if let blockReason = entry.attributes["blockReason"] {
      lines.append("Block reason: \(readableBlockReason(blockReason))")
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
    if let rankingScores = entry.attributes["memoryRankingScores"] {
      lines.append("Ranking scores: \(rankingScores)")
    }

    return TimelineContextReceiptSection(
      id: "memory",
      title: "Memory Context",
      body: lines.joined(separator: "\n")
    )
  }

  private static func compactionSection(_ entry: TimelineEntry) -> TimelineContextReceiptSection? {
    var lines: [String] = []

    appendBudgetLine(
      label: "Observation",
      sourceChars: entry.attributes["observationSourceChars"],
      budgetChars: entry.attributes["observationBudgetChars"],
      truncated: entry.attributes["observationTruncated"],
      to: &lines
    )
    appendBudgetLine(
      label: "Prompt",
      sourceChars: entry.attributes["promptSourceChars"],
      budgetChars: entry.attributes["promptBudgetChars"],
      truncated: entry.attributes["promptTruncated"],
      to: &lines
    )
    appendBudgetLine(
      label: "Prior observations",
      sourceChars: entry.attributes["priorObservationSourceChars"],
      budgetChars: entry.attributes["priorObservationBudgetChars"],
      truncated: entry.attributes["priorObservationTruncated"],
      to: &lines
    )

    if let priorCount = entry.attributes["priorObservationCount"] {
      lines.append("Prior observation count: \(priorCount)")
    }
    if let priorPaths = entry.attributes["priorObservationPaths"] {
      lines.append("Prior observation paths:\n\(priorPaths)")
    }
    appendMemoryCompactionDecision(entry.attributes, to: &lines)

    guard !lines.isEmpty else {
      return nil
    }

    return TimelineContextReceiptSection(
      id: "compaction",
      title: "Context Compaction",
      body: lines.joined(separator: "\n")
    )
  }

  private static func hasActionReceiptContext(_ attributes: [String: String]) -> Bool {
    attributes["actionReceiptSchema"] != nil
      || attributes["toolName"] != nil
      || attributes["tool"] != nil
      || attributes["agentToolName"] != nil
  }

  private static func hasWorkspaceContext(_ attributes: [String: String]) -> Bool {
    if attributes["relativePath"] != nil
      || attributes["query"] != nil
      || attributes["resultCount"] != nil
      || attributes["uniquePathCount"] != nil
      || attributes["nextRelativePath"] != nil
      || attributes["workspaceDisplayName"] != nil
    {
      return true
    }

    switch attributes["tool"] ?? attributes["agentToolName"] {
    case "read_file", "search_files", "list_directory", "write_file", "generate_diff":
      return true
    default:
      return false
    }
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
    case "autoApproved":
      return "auto approved"
    case "blocked":
      return "blocked by mode or permission"
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

  private static func readableBlockReason(_ value: String) -> String {
    switch value {
    case "readOnlyMode":
      return "read-only mode"
    case "missingPermission":
      return "missing permission"
    case "approvalUnavailable":
      return "approval unavailable"
    default:
      return value
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

  private static func appendBudgetLine(
    label: String,
    sourceChars: String?,
    budgetChars: String?,
    truncated: String?,
    to lines: inout [String]
  ) {
    guard sourceChars != nil || budgetChars != nil || truncated != nil else {
      return
    }

    lines.append(
      "\(label): \(sourceChars ?? "unknown")/\(budgetChars ?? "unknown") chars"
        + " | truncated \(yesNo(truncated ?? "unknown"))"
    )
  }

  private static func appendMemoryCompactionDecision(
    _ attributes: [String: String],
    to lines: inout [String]
  ) {
    guard attributes["memoryContextMode"] != nil else {
      return
    }

    let omittedCount = attributes["memoryContextOmittedNoteCount"] ?? "0"
    let truncatedCount = attributes["memoryContextTruncatedNoteCount"] ?? "0"
    if omittedCount == "0", truncatedCount == "0" {
      return
    }

    let candidateCount = attributes["memoryContextCandidateNoteCount"] ?? "unknown"
    let selectedCount = attributes["memoryNoteCount"] ?? "unknown"
    lines.append(
      "Memory decision: selected \(selectedCount)/\(candidateCount) notes"
        + " | omitted \(omittedCount) | truncated \(truncatedCount)"
    )
  }

  private static func appendLine(_ label: String, _ value: String?, to lines: inout [String]) {
    guard let value, !value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      return
    }
    lines.append("\(label): \(value)")
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

  private static func actionCardSummary(_ attributes: [String: String]) -> String? {
    guard hasActionReceiptContext(attributes) else {
      return nil
    }

    let tool = attributes["toolName"] ?? attributes["tool"]
    let mode = LocalExecutionSafetyModePresenter.compact(
      attributes["localExecutionSafetyMode"] ?? "askBeforeChange"
    )
    let approval = compactApprovalPolicy(attributes["actionApprovalPolicy"] ?? inferredApprovalPolicy(tool))
    if let blockReason = attributes["blockReason"] {
      return "Action \(approval) by \(readableBlockReason(blockReason))"
    }
    if let tool {
      return "\(tool) | \(mode) | \(approval)"
    }
    return "\(mode) | \(approval)"
  }

  private static func sourceCardSummary(_ attributes: [String: String]) -> String? {
    guard attributes["webSearchSourceMode"] != nil
      || attributes["sourceAttribution"] == "web_search"
    else {
      return nil
    }

    if attributes["pageFetchPerformed"] == "true" {
      return "Web sources verified"
    }
    if attributes["sourceSnapshotAvailable"] == "true" {
      return "Web snapshot retained"
    }
    return "Web search results"
  }

  private static func memoryCardSummary(_ attributes: [String: String]) -> String? {
    guard attributes["memoryContextMode"] != nil || attributes["memoryNoteCount"] != nil else {
      return nil
    }

    let noteCount = attributes["memoryNoteCount"] ?? "0"
    if let candidateCount = attributes["memoryContextCandidateNoteCount"] {
      return "Memory \(noteCount)/\(candidateCount)"
    }
    return "Memory \(noteCount)"
  }

  private static func compactionCardSummary(_ attributes: [String: String]) -> String? {
    let promptTruncated = attributes["promptTruncated"] == "true"
    let observationTruncated = attributes["observationTruncated"] == "true"
    let priorTruncated = attributes["priorObservationTruncated"] == "true"
    guard promptTruncated || observationTruncated || priorTruncated else {
      return nil
    }

    return "Context compacted"
  }

  private static func compactApprovalPolicy(_ value: String) -> String {
    switch value {
    case "autoApproved":
      return "auto approved"
    case "blocked":
      return "blocked"
    case "requiresApproval":
      return "needs approval"
    case "requiresPluginPermission":
      return "needs plugin permission"
    case "readOnlyAllowed":
      return "read-only"
    default:
      return value
    }
  }
}
