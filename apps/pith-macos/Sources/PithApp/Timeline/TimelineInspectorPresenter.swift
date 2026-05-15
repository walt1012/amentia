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

    appendPluginConnectorRecoverySummary(entry, to: &lines)
    appendPluginRunSummary(entry, to: &lines)
    appendPluginInstallSummary(entry, to: &lines)
    appendPluginRefreshSummary(entry, to: &lines)
    appendPluginLifecycleSummary(entry, to: &lines)
    appendPluginRunnerSetupSummary(entry, to: &lines)

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
      "pluginInstallStatus",
      "pluginRefreshStatus",
      "pluginLifecycleStatus",
      "runStatus",
      "runBlocker",
      "runRepairHint",
      "commandInput",
      "connectorId",
      "connectorIds",
      "connectorStatus",
      "connectorRepairHint",
      "sourcePath",
      "pluginSourcePath",
    ].contains { key in entry.attributes[key] != nil }
  }

  private static func connectorSummary(_ entry: TimelineEntry) -> String? {
    guard let connectorIDs = firstAttribute(entry, keys: [
      "connectorId",
      "connectorIds",
      "pluginRunnerConnectorId",
      "pluginRunnerConnectorIds",
    ]) else {
      return nil
    }

    let services = firstAttribute(entry, keys: [
      "connectorService",
      "connectorServices",
      "pluginRunnerConnectorServices",
    ]) ?? "unknown service"
    let stores = firstAttribute(entry, keys: [
      "credentialStore",
      "connectorCredentialStores",
      "pluginRunnerConnectorStores",
    ]) ?? "unknown store"
    let providers = firstAttribute(entry, keys: [
      "credentialProvider",
      "connectorCredentialProviders",
      "pluginRunnerCredentialProviders",
    ]) ?? "unknown provider"
    let bindings = firstAttribute(entry, keys: [
      "credentialBinding",
      "connectorSecretBindings",
      "pluginRunnerSecretBindings",
    ]) ?? "unknown binding"
    return "Connectors: \(connectorIDs) | \(services) | \(stores) | \(providers) "
      + "| \(bindings)"
  }

  private static func firstAttribute(_ entry: TimelineEntry, keys: [String]) -> String? {
    keys.compactMap { key in entry.attributes[key] }.first
  }

  private static func appendPluginInstallSummary(
    _ entry: TimelineEntry,
    to lines: inout [String]
  ) {
    guard let status = entry.attributes["pluginInstallStatus"] else {
      return
    }

    lines.append("Install: \(status)")
    if let blocker = entry.attributes["installBlocker"] {
      lines.append("Install blocker: \(blocker)")
    }
    if let repairHint = entry.attributes["installRepairHint"] {
      lines.append("Install repair: \(repairHint)")
    }
  }

  private static func appendPluginRefreshSummary(
    _ entry: TimelineEntry,
    to lines: inout [String]
  ) {
    guard let status = entry.attributes["pluginRefreshStatus"] else {
      return
    }

    let count = entry.attributes["pluginRefreshDiagnosticCount"] ?? "0"
    lines.append("Plugin refresh: \(status) | diagnostics \(count)")
  }

  private static func appendPluginConnectorRecoverySummary(
    _ entry: TimelineEntry,
    to lines: inout [String]
  ) {
    guard entry.attributes["connectorStatus"] != nil
      || entry.attributes["connectorRepairHint"] != nil
    else {
      return
    }

    if let status = entry.attributes["connectorStatus"] {
      lines.append("Connector status: \(status)")
    }
    if let repairHint = entry.attributes["connectorRepairHint"] {
      lines.append("Connector repair: \(repairHint)")
    }
  }

  private static func appendPluginRunSummary(
    _ entry: TimelineEntry,
    to lines: inout [String]
  ) {
    guard entry.attributes["runStatus"] != nil
      || entry.attributes["runBlocker"] != nil
      || entry.attributes["runRepairHint"] != nil
      || entry.attributes["commandInput"] != nil
    else {
      return
    }

    if let status = entry.attributes["runStatus"] {
      lines.append("Run: \(status)")
    }
    if let input = entry.attributes["commandInput"] {
      lines.append("Input: \(input)")
    }
    if let blocker = entry.attributes["runBlocker"] {
      lines.append("Run blocker: \(blocker)")
    }
    if let repairHint = entry.attributes["runRepairHint"] {
      lines.append("Run repair: \(repairHint)")
    }
  }

  private static func appendPluginLifecycleSummary(
    _ entry: TimelineEntry,
    to lines: inout [String]
  ) {
    guard let status = entry.attributes["pluginLifecycleStatus"] else {
      return
    }

    let operation = entry.attributes["pluginLifecycleOperation"] ?? "plugin"
    lines.append("Lifecycle: \(operation) | \(status)")
    if let blocker = entry.attributes["lifecycleBlocker"] {
      lines.append("Lifecycle blocker: \(blocker)")
    }
    if let repairHint = entry.attributes["lifecycleRepairHint"] {
      lines.append("Lifecycle repair: \(repairHint)")
    }
  }

  private static func appendPluginRunnerSetupSummary(
    _ entry: TimelineEntry,
    to lines: inout [String]
  ) {
    guard entry.attributes["pluginRunnerExecutionDriver"] != nil
      || entry.attributes["pluginRunnerEntrypoint"] != nil
    else {
      return
    }

    let driver = entry.attributes["pluginRunnerExecutionDriver"] ?? "unknown driver"
    let kind = entry.attributes["pluginRunnerExecutionKind"]
      ?? entry.attributes["executionKind"]
      ?? "unknown execution"
    let entrypoint = entry.attributes["pluginRunnerEntrypoint"] ?? "unknown entrypoint"
    lines.append("Runner: \(driver) | \(kind) | \(entrypoint)")

    if let check = entry.attributes["pluginRunnerEntrypointCheck"] {
      let fileKind = entry.attributes["pluginRunnerEntrypointFileKind"] ?? "unknown file"
      let executable = entry.attributes["pluginRunnerEntrypointExecutable"] ?? "unknown"
      lines.append("Runner entrypoint: \(check) | \(fileKind) | executable \(executable)")
    }
    if let resolvedEntrypoint = entry.attributes["pluginRunnerResolvedEntrypoint"] {
      lines.append("Runner path: \(resolvedEntrypoint)")
    }
    if let pluginRoot = entry.attributes["pluginRunnerPluginRoot"] {
      lines.append("Plugin root: \(pluginRoot)")
    }
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

    if let serverCommand = entry.attributes["mcpServerCommand"] {
      lines.append("MCP server command: \(serverCommand)")
    }
    if let errorCode = entry.attributes["mcpErrorCode"] {
      lines.append("MCP error code: \(errorCode)")
    }
    if let structuredContentStatus = entry.attributes["mcpStructuredContentStatus"] {
      lines.append("MCP structured content: \(structuredContentStatus)")
    }
    if let contentStatus = entry.attributes["mcpContentStatus"] {
      lines.append("MCP content: \(contentStatus)")
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
