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
      let networkAllowed = entry.attributes["sandboxNetworkAllowed"] ?? "unknown"
      lines.append(
        "Sandbox: \(mode) | backend \(backend) | active \(active) | network \(networkAllowed)"
      )
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

    return lines.joined(separator: "\n")
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
