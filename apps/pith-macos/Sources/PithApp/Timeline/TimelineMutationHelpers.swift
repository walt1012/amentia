import Foundation

struct TimelineVisibleState {
  let timeline: [TimelineEntry]
  let selectedEntryID: TimelineEntry.ID?
}

struct ActiveTurnSelectionState {
  let activeTurnID: String?
  let activeTurnThreadID: String?
}

enum TimelineMutationState {
  static func pendingApprovalIDs(
    from approvals: [RuntimeBridge.RuntimeApproval]
  ) -> Set<String> {
    Set(approvals.map(\.id))
  }

  static func threadsByRefreshingPreview(
    _ threads: [ThreadSummary],
    threadID: String,
    preview: String
  ) -> [ThreadSummary] {
    var refreshedThreads = threads
    guard let index = refreshedThreads.firstIndex(where: { $0.id == threadID }) else {
      return refreshedThreads
    }

    refreshedThreads[index].preview = preview
    return refreshedThreads
  }

  static func threadTimelines(
    for workspaceThreads: [ThreadSummary],
    existingTimelines: [String: [TimelineEntry]]
  ) -> [String: [TimelineEntry]] {
    Dictionary(
      uniqueKeysWithValues: workspaceThreads.map { thread in
        (
          thread.id,
          existingTimelines[thread.id]
            ?? TimelineEntryFactory.defaultTimeline(for: thread.title)
        )
      }
    )
  }

  static func pendingApprovalIDsByRetainingWorkspaceThreads(
    _ pendingApprovalIDs: [String: Set<String>],
    workspaceThreads: [ThreadSummary]
  ) -> [String: Set<String>] {
    let workspaceThreadIDs = Set(workspaceThreads.map(\.id))
    return pendingApprovalIDs.filter { workspaceThreadIDs.contains($0.key) }
  }

  static func selectedThreadID(
    workspaceThreads: [ThreadSummary],
    currentSelectionID: ThreadSummary.ID?
  ) -> ThreadSummary.ID? {
    workspaceThreads.first(where: { $0.id == currentSelectionID })?.id
      ?? workspaceThreads.first?.id
  }

  static func entriesByAppending(
    entry: TimelineEntry,
    existingEntries: [TimelineEntry]?,
    fallbackTitle: String
  ) -> [TimelineEntry] {
    var entries = existingEntries ?? TimelineEntryFactory.defaultTimeline(for: fallbackTitle)
    entries.insert(entry, at: 0)
    return entries
  }

  static func visibleTimeline(
    selectedThreadID: ThreadSummary.ID?,
    threadTimelines: [String: [TimelineEntry]],
    threads: [ThreadSummary],
    previousSelectionID: TimelineEntry.ID?
  ) -> TimelineVisibleState {
    guard let selectedThreadID else {
      return TimelineVisibleState(timeline: [], selectedEntryID: nil)
    }

    let timeline = threadTimelines[selectedThreadID]
      ?? TimelineEntryFactory.defaultTimeline(
        for: TimelineSessionState.threadTitle(for: selectedThreadID, threads: threads)
      )

    return TimelineVisibleState(
      timeline: timeline,
      selectedEntryID: TimelineEntryFactory.bestSelectionID(
        previousSelectionID: previousSelectionID,
        entries: timeline
      )
    )
  }

  static func visibleTimelineUpdate(
    updatedThreadID: ThreadSummary.ID,
    selectedThreadID: ThreadSummary.ID?,
    entries: [TimelineEntry],
    previousSelectionID: TimelineEntry.ID?
  ) -> TimelineVisibleState? {
    guard selectedThreadID == updatedThreadID else {
      return nil
    }

    return TimelineVisibleState(
      timeline: entries,
      selectedEntryID: TimelineEntryFactory.bestSelectionID(
        previousSelectionID: previousSelectionID,
        entries: entries
      )
    )
  }

  static func activeTurnSelection(
    currentActiveTurnID: String?,
    currentActiveTurnThreadID: String?,
    threadID: String,
    runtimeActiveTurnID: String?
  ) -> ActiveTurnSelectionState {
    guard let runtimeActiveTurnID else {
      if currentActiveTurnThreadID == threadID {
        return ActiveTurnSelectionState(activeTurnID: nil, activeTurnThreadID: nil)
      }

      return ActiveTurnSelectionState(
        activeTurnID: currentActiveTurnID,
        activeTurnThreadID: currentActiveTurnThreadID
      )
    }

    if currentActiveTurnID == runtimeActiveTurnID,
       currentActiveTurnThreadID == threadID
    {
      return ActiveTurnSelectionState(
        activeTurnID: currentActiveTurnID,
        activeTurnThreadID: currentActiveTurnThreadID
      )
    }

    return ActiveTurnSelectionState(
      activeTurnID: runtimeActiveTurnID,
      activeTurnThreadID: threadID
    )
  }
}
