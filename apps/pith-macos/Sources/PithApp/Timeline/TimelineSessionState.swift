import Foundation

struct WelcomeTimelineState {
  let thread: ThreadSummary
  let timeline: [TimelineEntry]
}

enum TimelineSessionState {
  static let welcomeThreadID = "local-welcome"

  static func welcomeState() -> WelcomeTimelineState {
    WelcomeTimelineState(
      thread: ThreadSummary(
        id: welcomeThreadID,
        title: "Welcome to Pith",
        preview: "Open a workspace to begin local work.",
        workspaceRootPath: nil,
        workspaceDisplayName: nil
      ),
      timeline: TimelineEntryFactory.welcomeTimeline()
    )
  }

  static func threadTitle(for threadID: String, threads: [ThreadSummary]) -> String {
    threads.first(where: { $0.id == threadID })?.title ?? "Thread"
  }

  static func selectedEntry(
    selectedEntryID: TimelineEntry.ID?,
    timeline: [TimelineEntry]
  ) -> TimelineEntry? {
    guard let selectedEntryID else {
      return nil
    }

    return timeline.first(where: { $0.id == selectedEntryID })
  }

  static func hasRuntimeThreadSelection(
    selectedThreadID: ThreadSummary.ID?,
    threads: [ThreadSummary],
    workspace: WorkspaceSummary?
  ) -> Bool {
    guard let selectedThreadID,
          !selectedThreadID.hasPrefix("local-"),
          let selectedThread = threads.first(where: { $0.id == selectedThreadID }),
          let workspace
    else {
      return false
    }

    return selectedThread.workspaceRootPath == workspace.rootPath
  }

  static func isWaitingForFirstMessage(
    selectedThreadID: ThreadSummary.ID?,
    threadTimelines: [String: [TimelineEntry]],
    visibleTimeline: [TimelineEntry]
  ) -> Bool {
    guard let selectedThreadID,
          !selectedThreadID.hasPrefix("local-")
    else {
      return false
    }

    let entries = threadTimelines[selectedThreadID] ?? visibleTimeline
    return !entries.contains(where: isUserStartedTimelineEntry)
  }

  private static func isUserStartedTimelineEntry(_ entry: TimelineEntry) -> Bool {
    switch entry.kind {
    case .userMessage, .assistantMessage, .plan, .tool, .diff, .approval:
      return true
    case .system, .warning:
      return false
    }
  }
}
