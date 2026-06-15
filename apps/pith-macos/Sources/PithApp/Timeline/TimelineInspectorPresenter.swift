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

    let typeLine = "Type: \(readableStatus(entry.kind.rawValue))"
    if entry.attributes.isEmpty {
      return typeLine
    }

    let detail = entry.attributes
      .sorted(by: { $0.key < $1.key })
      .map { "\($0.key): \($0.value)" }
      .joined(separator: "\n")

    return "\(typeLine)\n\(detail)"
  }

  static func selectedEntrySourceSummary(_ snapshot: TimelineInspectorSnapshot) -> String? {
    TimelineContextReceiptPresenter.sourceSummary(snapshot)
  }

  static func selectedEntryActionReceiptSummary(_ snapshot: TimelineInspectorSnapshot) -> String? {
    TimelineContextReceiptPresenter.actionSummary(snapshot)
  }

  static func selectedEntryContextReceiptSections(
    _ snapshot: TimelineInspectorSnapshot
  ) -> [TimelineContextReceiptSection] {
    TimelineContextReceiptPresenter.sections(snapshot)
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
    TimelineContextReceiptPresenter.memorySummary(snapshot)
  }

  static func selectedEntryPluginSummary(_ snapshot: TimelineInspectorSnapshot) -> String? {
    guard let entry = snapshot.selectedEntry, hasPluginContext(entry) else {
      return nil
    }

    var lines: [String] = []
    if let displayName = entry.attributes["pluginDisplayName"] {
      lines.append("Plugin: \(displayName)")
    } else if entry.attributes["pluginId"] != nil {
      lines.append("Plugin: available")
    }

    if let commandID = entry.attributes["commandId"] {
      lines.append("Action: \(readableIdentifier(commandID))")
    }

    if entry.attributes["approvalId"] != nil {
      let action = entry.attributes["action"] ?? "unknown"
      lines.append("Approval: \(readableIdentifier(action))")
    }

    lines.append(contentsOf: TimelineConnectorEvidencePresenter.summaryLines(
      attributes: entry.attributes
    ))
    appendPluginConnectorRecoverySummary(entry, to: &lines)
    appendPluginRunSummary(entry, to: &lines)
    appendPluginInstallSummary(entry, to: &lines)
    appendPluginRefreshSummary(entry, to: &lines)
    appendPluginLifecycleSummary(entry, to: &lines)
    appendPluginRunnerSetupSummary(entry, to: &lines)

    if entry.attributes["permissionGate"] != nil {
      let required = entry.attributes["requiredPermissionLabel"]
        ?? entry.attributes["requiredPermission"].map(readablePermission)
        ?? "unknown"
      lines.append("Permission needed: \(required)")
      if let recoveryHint = entry.attributes["permissionRecoveryHint"] {
        lines.append("Fix: \(recoveryHint)")
      }
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
      let active = entry.attributes["sandboxActive"] ?? "unknown"
      let networkPolicy = entry.attributes["sandboxNetworkPolicy"]
        ?? sandboxNetworkPolicySummary(entry.attributes["sandboxNetworkAllowed"])
      lines.append(
        "Sandbox: \(readableStatus(active)) | \(networkPolicy) | mode \(mode)"
      )

      if entry.attributes["sandboxTempRoot"] != nil {
        lines.append("Temporary files stayed inside the selected project.")
      }

      if entry.attributes["sandboxWritableRoots"] != nil {
        lines.append("Writes were limited to approved project locations.")
      }

      if let detail = entry.attributes["sandboxDetail"] {
        lines.append("Detail: \(detail)")
      }
    }

    if let outputContextMode = entry.attributes["sandboxOutputContextMode"] {
      lines.append(
        "Output: \(outputContextMode). Long command output was condensed for context."
      )
    }

    if entry.attributes["sandboxOutputArtifactDirectory"] != nil {
      lines.append("Full output was saved for troubleshooting.")
    }
    if entry.attributes["sandboxOutputArtifactsTruncated"] == "true",
       let artifactLimit = entry.attributes["sandboxOutputArtifactMaxBytesPerStream"]
    {
      lines.append("Saved output was capped at \(artifactLimit) bytes per stream.")
    }
    if entry.attributes["sandboxOutputStdoutArtifactPath"] != nil {
      lines.append("Captured standard output is available in Support Details.")
    }
    if entry.attributes["sandboxOutputStderrArtifactPath"] != nil {
      lines.append("Captured error output is available in Support Details.")
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
      "nextCommandId",
      "nextCommandInput",
      "nextCommandInputHint",
      "nextCommandInputTemplate",
      "nextCommandLabel",
      "retryCommandId",
      "retryInput",
      "retryInputEditable",
      "retryInputHint",
      "sourcePath",
      "pluginSourcePath",
    ].contains { key in entry.attributes[key] != nil }
      || TimelineConnectorEvidencePresenter.hasEvidence(attributes: entry.attributes)
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
    lines.append("Plugin refresh: \(status) | setup notes \(count)")
    if let repairHint = entry.attributes["pluginRefreshRepairHint"] {
      lines.append("Plugin refresh repair: \(repairHint)")
    }
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
      lines.append("Connection status: \(readableStatus(status))")
    }
    if let repairHint = entry.attributes["connectorRepairHint"] {
      lines.append("Connection fix: \(repairHint)")
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
      lines.append("Blocked: \(blocker)")
    }
    if let repairHint = entry.attributes["runRepairHint"] {
      lines.append("Fix: \(repairHint)")
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
    lines.append("Plugin operation: \(readableIdentifier(operation)) | \(readableStatus(status))")
    if let blocker = entry.attributes["lifecycleBlocker"] {
      lines.append("Blocked: \(blocker)")
    }
    if let repairHint = entry.attributes["lifecycleRepairHint"] {
      lines.append("Fix: \(repairHint)")
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

    let kind = entry.attributes["pluginRunnerExecutionKind"]
      ?? entry.attributes["executionKind"]
      ?? "local action"
    lines.append("Plugin runner: \(readableIdentifier(kind))")

    if let setupStatus = entry.attributes["pluginRunnerSetupStatus"] {
      let phase = entry.attributes["pluginRunnerSetupPhase"] ?? "unknown"
      lines.append("Runner setup: \(readableStatus(setupStatus)) | \(readableIdentifier(phase))")
    }
    if let check = entry.attributes["pluginRunnerEntrypointCheck"] {
      lines.append("Runner file check: \(readableStatus(check))")
    }
    if entry.attributes["pluginRunnerResolvedEntrypoint"] != nil
      || entry.attributes["pluginRunnerPluginRoot"] != nil
    {
      lines.append("Runner paths are available in Support Details.")
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
      "Plugin runner: \(readableIdentifier(failureKind)) | \(readableIdentifier(reason)) | "
        + "status \(readableStatus(status)) | exit \(code)"
    )

    if let errorCode = entry.attributes["pluginRunnerErrorCode"] {
      lines.append("Runner error: \(readableIdentifier(errorCode))")
    }
    if let recoveryHint = entry.attributes["pluginRunnerRecoveryHint"] {
      lines.append("Recovery: \(recoveryHint)")
    }

    if entry.attributes["pluginRunnerStdoutRetainedBytes"] != nil
      || entry.attributes["pluginRunnerStderrRetainedBytes"] != nil
    {
      lines.append("Runner output was condensed for context.")
    }

    if let stderrPreview = entry.attributes["pluginRunnerStderrPreview"] {
      lines.append("Runner error preview:\n\(stderrPreview)")
    }
    if let stdoutPreview = entry.attributes["pluginRunnerStdoutPreview"] {
      lines.append("Runner output preview:\n\(stdoutPreview)")
    }
  }

  private static func appendMcpProtocolSummary(
    _ entry: TimelineEntry,
    to lines: inout [String]
  ) {
    guard let protocolStatus = entry.attributes["mcpProtocolStatus"] else {
      return
    }

    let server = entry.attributes["mcpServerId"].map(readableIdentifier) ?? "unknown"
    let tool = entry.attributes["mcpToolName"].map(readableIdentifier) ?? "unknown"
    lines.append(
      "MCP: \(readableStatus(protocolStatus)) | server \(server) | tool \(tool)"
    )

    if entry.attributes["mcpServerCommand"] != nil {
      lines.append("MCP launch command is available in Support Details.")
    }
    if let errorCode = entry.attributes["mcpErrorCode"] {
      lines.append("MCP error: \(readableIdentifier(errorCode))")
    }
    if let structuredContentStatus = entry.attributes["mcpStructuredContentStatus"] {
      lines.append("MCP structured content: \(structuredContentStatus)")
    }
    if let contentStatus = entry.attributes["mcpContentStatus"] {
      lines.append("MCP content: \(contentStatus)")
    }
    if let invalidPreview = entry.attributes["mcpLastInvalidJsonPreview"] {
      lines.append("MCP invalid response preview:\n\(invalidPreview)")
    }
  }

  private static func readableIdentifier(_ value: String) -> String {
    let tail = value.components(separatedBy: "::").last ?? value
    let words = tail
      .split { character in
        character == "." || character == "_" || character == "-" || character == ":"
      }
      .map { word in
        let lowercased = word.lowercased()
        return lowercased.prefix(1).uppercased() + String(lowercased.dropFirst())
      }

    return words.isEmpty ? value : words.joined(separator: " ")
  }

  private static func readablePermission(_ value: String) -> String {
    if value.hasPrefix("tool:") {
      return readableIdentifier(String(value.dropFirst("tool:".count)))
    }
    if value.hasPrefix("permission:") {
      return readableIdentifier(String(value.dropFirst("permission:".count)))
    }
    return readableIdentifier(value)
  }

  private static func readableStatus(_ value: String) -> String {
    switch value {
    case "true":
      return "active"
    case "false":
      return "inactive"
    case "success", "completed", "ready":
      return "ready"
    case "notSent", "notRequested":
      return "not sent yet"
    default:
      return readableIdentifier(value).lowercased()
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
