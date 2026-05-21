import Foundation

enum TimelineEntryFactory {
  static func entry(
    kind: TimelineEntry.Kind,
    title: String,
    body: String,
    attributes: [String: String] = [:]
  ) -> TimelineEntry {
    TimelineEntry(
      id: UUID().uuidString,
      kind: kind,
      title: title,
      body: body,
      attributes: attributes
    )
  }

  static func system(
    title: String,
    body: String,
    attributes: [String: String] = [:]
  ) -> TimelineEntry {
    entry(kind: .system, title: title, body: body, attributes: attributes)
  }

  static func warning(
    title: String,
    body: String,
    attributes: [String: String] = [:]
  ) -> TimelineEntry {
    entry(kind: .warning, title: title, body: body, attributes: attributes)
  }

  static func welcomeTimeline() -> [TimelineEntry] {
    [
      TimelineEntry(
        id: "welcome-start-local-setup",
        kind: .system,
        title: "Start Local Setup",
        body: "Launch the runtime, choose a local model, open a workspace, create or select a thread, then send one short local request.",
        attributes: [
          "path": "runtime -> model -> workspace -> thread -> first request"
        ]
      ),
      TimelineEntry(
        id: "welcome-local-first-work",
        kind: .assistantMessage,
        title: "Local-First Work",
        body:
          "Pith works against local workspaces and does not call external model APIs for core responses.",
        attributes: [
          "model": "local"
        ]
      ),
    ]
  }

  static func defaultTimeline(for title: String) -> [TimelineEntry] {
    [
      TimelineEntry(
        id: "default-thread-ready:\(title)",
        kind: .system,
        title: "Thread Ready",
        body: "\(title) is ready after runtime, model, workspace, and thread setup. Send one short request to finish first-use setup.",
        attributes: [
          "setup": "runtime, model, workspace, thread, first request"
        ]
      ),
    ]
  }

  static func transientEntries(
    from items: [RuntimeBridge.RuntimeTimelineItemResult]
  ) -> [TimelineEntry] {
    items.map { item in
      TimelineEntry(
        id: UUID().uuidString,
        kind: kind(for: item.kind),
        title: item.title,
        body: item.content,
        attributes: item.attributes
      )
    }
  }

  static func runtimeEntries(
    from items: [RuntimeBridge.RuntimeTimelineItemResult]
  ) -> [TimelineEntry] {
    items.enumerated().map { index, item in
      TimelineEntry(
        id: stableRuntimeID(for: item, index: index),
        kind: kind(for: item.kind),
        title: item.title,
        body: item.content,
        attributes: item.attributes
      )
    }
  }

  static func runtimeEntries(
    from items: [RuntimeBridge.RuntimeTimelineItemResult],
    existingEntries: [TimelineEntry]?,
    fallbackTitle: String
  ) -> [TimelineEntry] {
    let entries = runtimeEntries(from: items)
    if entries.isEmpty {
      if let existingEntries, !existingEntries.isEmpty {
        return existingEntries
      }

      return defaultTimeline(for: fallbackTitle)
    }

    return entries
  }

  static func bestSelectionID(
    previousSelectionID: TimelineEntry.ID?,
    entries: [TimelineEntry]
  ) -> TimelineEntry.ID? {
    if let previousSelectionID,
       entries.contains(where: { $0.id == previousSelectionID }) {
      return previousSelectionID
    }

    return entries.first?.id
  }

  private static func kind(for rawKind: String) -> TimelineEntry.Kind {
    switch rawKind {
    case "userMessage":
      return .userMessage
    case "assistantMessage":
      return .assistantMessage
    case "plan":
      return .plan
    case "diffArtifact":
      return .diff
    case "toolStart", "toolResult", "pluginCommand", "pluginResult":
      return .tool
    case "approvalRequested", "approvalResolved":
      return .approval
    case "warning":
      return .warning
    default:
      return .system
    }
  }

  private static func stableRuntimeID(
    for item: RuntimeBridge.RuntimeTimelineItemResult,
    index: Int
  ) -> String {
    if let approvalID = item.attributes["approvalId"] {
      return "approval:\(approvalID):\(item.kind):\(item.title)"
    }
    if let turnID = item.attributes["turnId"] {
      return "turn:\(turnID):\(item.kind):\(item.title)"
    }
    if let agentStepID = item.attributes["agentStepId"] {
      return "agent-step:\(agentStepID):\(item.kind):\(item.title)"
    }
    if let toolCallID = item.attributes["toolCallId"] {
      return "tool-call:\(toolCallID):\(item.kind):\(item.title)"
    }
    return "runtime:\(index):\(item.kind):\(item.title)"
  }
}
