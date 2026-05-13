import Foundation

struct TimelineInspectorSnapshot {
  let selectedEntry: TimelineEntry?
}

enum TimelineInspectorPresenter {
  static func selectedEntryTitle(_ snapshot: TimelineInspectorSnapshot) -> String {
    snapshot.selectedEntry?.title ?? "No Item Selected"
  }

  static func selectedEntryBody(_ snapshot: TimelineInspectorSnapshot) -> String {
    snapshot.selectedEntry?.body ?? "Select a timeline item to inspect its details."
  }

  static func selectedEntryMetadata(_ snapshot: TimelineInspectorSnapshot) -> String {
    guard let entry = snapshot.selectedEntry else {
      return "No timeline item is selected."
    }

    if entry.attributes.isEmpty {
      return entry.kind.rawValue
    }

    let detail = entry.attributes
      .sorted(by: { $0.key < $1.key })
      .map { "\($0.key): \($0.value)" }
      .joined(separator: "\n")

    return "\(entry.kind.rawValue)\n\(detail)"
  }

  static func selectedDiffSummary(_ snapshot: TimelineInspectorSnapshot) -> String? {
    guard let entry = snapshot.selectedEntry, entry.kind == .diff else {
      return nil
    }

    let lines = diffLines(from: entry.body)
    let additions = lines.filter { $0.kind == .addition }.count
    let deletions = lines.filter { $0.kind == .deletion }.count
    let hunks = lines.filter { $0.kind == .hunk }.count
    let path = entry.attributes["relativePath"] ?? diffPathSummary(from: lines)
    return "\(path) | +\(additions) -\(deletions) | \(hunks) hunk\(hunks == 1 ? "" : "s")"
  }

  static func selectedDiffLines(_ snapshot: TimelineInspectorSnapshot) -> [DiffLineSummary] {
    guard let entry = snapshot.selectedEntry, entry.kind == .diff else {
      return []
    }

    return diffLines(from: entry.body)
  }

  static func selectedEntryMemorySummary(_ snapshot: TimelineInspectorSnapshot) -> String? {
    guard let entry = snapshot.selectedEntry else {
      return nil
    }

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

    return lines.joined(separator: "\n")
  }

  static func selectedEntryPluginSummary(_ snapshot: TimelineInspectorSnapshot) -> String? {
    guard let entry = snapshot.selectedEntry, hasPluginContext(entry) else {
      return nil
    }

    var lines: [String] = []
    if let pluginID = entry.attributes["pluginId"] {
      let displayName = entry.attributes["pluginDisplayName"] ?? "Plugin"
      lines.append("Plugin: \(displayName) | \(pluginID)")
    }

    if let commandID = entry.attributes["commandId"] {
      let executionKind = entry.attributes["executionKind"] ?? "unknown execution"
      lines.append("Command: \(commandID) | \(executionKind)")
    }

    if let runID = entry.attributes["pluginCommandRunId"] {
      lines.append("Run: \(runID)")
    }

    if let sourcePath = entry.attributes["sourcePath"] {
      lines.append("Source: \(sourcePath)")
    }

    if let approvalID = entry.attributes["approvalId"] {
      let action = entry.attributes["action"] ?? "unknown"
      lines.append("Approval: \(action) | \(approvalID)")
    }

    if let connectorSummary = connectorSummary(entry) {
      lines.append(connectorSummary)
    }

    if let permissionGate = entry.attributes["permissionGate"] {
      let required = entry.attributes["requiredPermission"] ?? "unknown"
      lines.append("Permission gate: \(permissionGate) | requires \(required)")
    }

    appendPluginRunnerSummary(entry, to: &lines)
    appendMcpProtocolSummary(entry, to: &lines)

    return lines.joined(separator: "\n")
  }

  static func selectedEntrySandboxSummary(_ snapshot: TimelineInspectorSnapshot) -> String? {
    guard let entry = snapshot.selectedEntry else {
      return nil
    }

    let hasSandbox = entry.attributes["sandboxMode"] != nil
    let hasOutputContext = entry.attributes["sandboxOutputContextMode"] != nil
    if !hasSandbox && !hasOutputContext {
      return nil
    }

    var lines: [String] = []
    if hasSandbox {
      let mode = entry.attributes["sandboxMode"] ?? "unknown"
      let backend = entry.attributes["sandboxBackend"] ?? "unknown"
      let active = entry.attributes["sandboxActive"] ?? "unknown"
      let networkPolicy = entry.attributes["sandboxNetworkPolicy"]
        ?? sandboxNetworkPolicySummary(entry.attributes["sandboxNetworkAllowed"])
      lines.append(
        "Sandbox: \(mode) | backend \(backend) | active \(active) | \(networkPolicy)"
      )

      if let temporaryRoot = entry.attributes["sandboxTempRoot"] {
        lines.append("Temp root: \(temporaryRoot)")
      }

      if let writableRoots = entry.attributes["sandboxWritableRoots"] {
        lines.append("Writable roots:\n\(writableRoots)")
      }

      if let detail = entry.attributes["sandboxDetail"] {
        lines.append("Detail: \(detail)")
      }
    }

    if let outputContextMode = entry.attributes["sandboxOutputContextMode"] {
      let retainedStdout = entry.attributes["sandboxOutputRetainedStdoutBytes"] ?? "unknown"
      let sourceStdout = entry.attributes["sandboxOutputSourceStdoutBytes"] ?? "unknown"
      let retainedStderr = entry.attributes["sandboxOutputRetainedStderrBytes"] ?? "unknown"
      let sourceStderr = entry.attributes["sandboxOutputSourceStderrBytes"] ?? "unknown"
      let savedBytes = entry.attributes["sandboxOutputSavedBytes"] ?? "unknown"
      let savingsPercent = entry.attributes["sandboxOutputSavingsPercent"] ?? "unknown"
      lines.append(
        "Output: \(outputContextMode) | stdout \(retainedStdout)/\(sourceStdout) bytes | "
          + "stderr \(retainedStderr)/\(sourceStderr) bytes | "
          + "saved \(savedBytes) bytes (\(savingsPercent)%)"
      )
    }

    if let artifactDirectory = entry.attributes["sandboxOutputArtifactDirectory"] {
      lines.append("Artifact: \(artifactDirectory)")
    }
    if let stdoutArtifact = entry.attributes["sandboxOutputStdoutArtifactPath"] {
      lines.append("Full stdout: \(stdoutArtifact)")
    }
    if let stderrArtifact = entry.attributes["sandboxOutputStderrArtifactPath"] {
      lines.append("Full stderr: \(stderrArtifact)")
    }

    return lines.joined(separator: "\n")
  }

  private static func hasPluginContext(_ entry: TimelineEntry) -> Bool {
    [
      "pluginId",
      "commandId",
      "executionKind",
      "approvalId",
      "pluginRunnerExitReason",
      "pluginRunnerErrorCode",
      "pluginRunnerFailureKind",
      "pluginRunnerRecoveryHint",
      "mcpProtocolStatus",
      "permissionGate",
      "pluginCommandRunId",
    ].contains { key in entry.attributes[key] != nil }
  }

  private static func connectorSummary(_ entry: TimelineEntry) -> String? {
    guard let connectorIDs = firstAttribute(entry, keys: [
      "connectorIds",
      "pluginRunnerConnectorIds",
    ]) else {
      return nil
    }

    let services = firstAttribute(entry, keys: [
      "connectorServices",
      "pluginRunnerConnectorServices",
    ]) ?? "unknown service"
    let stores = firstAttribute(entry, keys: [
      "connectorCredentialStores",
      "pluginRunnerConnectorStores",
    ]) ?? "unknown store"
    let providers = firstAttribute(entry, keys: [
      "connectorCredentialProviders",
      "pluginRunnerCredentialProviders",
    ]) ?? "unknown provider"
    return "Connectors: \(connectorIDs) | \(services) | \(stores) | \(providers)"
  }

  private static func firstAttribute(_ entry: TimelineEntry, keys: [String]) -> String? {
    keys.compactMap { key in entry.attributes[key] }.first
  }

  private static func appendPluginRunnerSummary(
    _ entry: TimelineEntry,
    to lines: inout [String]
  ) {
    guard entry.attributes["pluginRunnerExitReason"] != nil
      || entry.attributes["pluginRunnerErrorCode"] != nil
    else {
      return
    }

    let reason = entry.attributes["pluginRunnerExitReason"] ?? "unknown"
    let status = entry.attributes["pluginRunnerExitStatus"] ?? "unknown"
    let code = entry.attributes["pluginRunnerExitCode"] ?? "unknown"
    let failureKind = entry.attributes["pluginRunnerFailureKind"] ?? "unknown"
    lines.append(
      "Plugin runner: \(failureKind) | \(reason) | status \(status) | exit \(code)"
    )

    if let errorCode = entry.attributes["pluginRunnerErrorCode"] {
      lines.append("Plugin runner error: \(errorCode)")
    }
    if let recoveryHint = entry.attributes["pluginRunnerRecoveryHint"] {
      lines.append("Recovery: \(recoveryHint)")
    }

    let retainedStdout = entry.attributes["pluginRunnerStdoutRetainedBytes"] ?? "unknown"
    let sourceStdout = entry.attributes["pluginRunnerStdoutSourceBytes"] ?? "unknown"
    let retainedStderr = entry.attributes["pluginRunnerStderrRetainedBytes"] ?? "unknown"
    let sourceStderr = entry.attributes["pluginRunnerStderrSourceBytes"] ?? "unknown"
    lines.append(
      "Runner output: stdout \(retainedStdout)/\(sourceStdout) bytes | "
        + "stderr \(retainedStderr)/\(sourceStderr) bytes"
    )

    if let stderrPreview = entry.attributes["pluginRunnerStderrPreview"] {
      lines.append("Runner stderr preview:\n\(stderrPreview)")
    }
    if let stdoutPreview = entry.attributes["pluginRunnerStdoutPreview"] {
      lines.append("Runner stdout preview:\n\(stdoutPreview)")
    }
  }

  private static func appendMcpProtocolSummary(
    _ entry: TimelineEntry,
    to lines: inout [String]
  ) {
    guard let protocolStatus = entry.attributes["mcpProtocolStatus"] else {
      return
    }

    let server = entry.attributes["mcpServerId"] ?? "unknown"
    let tool = entry.attributes["mcpToolName"] ?? "unknown"
    let initializeSeen = entry.attributes["mcpInitializeResponseSeen"] ?? "unknown"
    let toolSeen = entry.attributes["mcpToolResponseSeen"] ?? "unknown"
    let invalidLines = entry.attributes["mcpInvalidJsonLineCount"] ?? "0"
    lines.append(
      "MCP: \(protocolStatus) | server \(server) | tool \(tool) | "
        + "initialize \(initializeSeen) | tool response \(toolSeen) | invalid stdout \(invalidLines)"
    )

    if let errorCode = entry.attributes["mcpErrorCode"] {
      lines.append("MCP error code: \(errorCode)")
    }
    if let invalidPreview = entry.attributes["mcpLastInvalidJsonPreview"] {
      lines.append("MCP invalid stdout preview:\n\(invalidPreview)")
    }
  }

  private static func sandboxNetworkPolicySummary(_ value: String?) -> String {
    switch value {
    case "true":
      return "network allowed"
    case "false":
      return "network denied"
    default:
      return "network unknown"
    }
  }

  private static func diffLines(from body: String) -> [DiffLineSummary] {
    body
      .components(separatedBy: .newlines)
      .enumerated()
      .map { index, line in
        DiffLineSummary(
          id: "\(index)",
          lineNumber: index + 1,
          text: line,
          kind: diffLineKind(for: line)
        )
      }
  }

  private static func diffLineKind(for line: String) -> DiffLineKind {
    if line.hasPrefix("@@") {
      return .hunk
    }

    if line.hasPrefix("diff --git")
      || line.hasPrefix("index ")
      || line.hasPrefix("+++")
      || line.hasPrefix("---")
    {
      return .metadata
    }

    if line.hasPrefix("+") {
      return .addition
    }

    if line.hasPrefix("-") {
      return .deletion
    }

    return .context
  }

  private static func diffPathSummary(from lines: [DiffLineSummary]) -> String {
    let pathLine = lines.first { line in
      line.text.hasPrefix("+++ b/") || line.text.hasPrefix("--- a/")
    }

    guard let pathLine else {
      return "Diff"
    }

    return pathLine.text
      .replacingOccurrences(of: "+++ b/", with: "")
      .replacingOccurrences(of: "--- a/", with: "")
  }
}
