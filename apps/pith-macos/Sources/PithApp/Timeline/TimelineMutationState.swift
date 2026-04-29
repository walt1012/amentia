import Foundation

struct TimelineRuntimeState {
  var threads: [ThreadSummary]
  var selectedThreadID: ThreadSummary.ID?
  var timeline: [TimelineEntry]
  var selectedEntryID: TimelineEntry.ID?
  var activeTurnID: String?
  var threadTimelines: [String: [TimelineEntry]]
  var threadPendingApprovalIDs: [String: Set<String>]
  var activeTurnThreadID: String?
  var announcedSetupCompleteThreadIDs: Set<String>

  init(welcomeState: WelcomeTimelineState) {
    let welcomeThread = welcomeState.thread
    let welcomeTimeline = welcomeState.timeline

    self.threads = [welcomeThread]
    self.selectedThreadID = welcomeThread.id
    self.timeline = welcomeTimeline
    self.selectedEntryID = welcomeTimeline.first?.id
    self.activeTurnID = nil
    self.threadTimelines = [welcomeThread.id: welcomeTimeline]
    self.threadPendingApprovalIDs = [:]
    self.activeTurnThreadID = nil
    self.announcedSetupCompleteThreadIDs = Set<String>()
  }

  var selectedPendingApprovalIDs: Set<String> {
    selectedThreadID.map {
      threadPendingApprovalIDs[$0, default: Set<String>()]
    } ?? Set<String>()
  }

  var hasCancelableRuntimeTurn: Bool {
    activeTurnID != nil && activeTurnThreadID != nil
  }

  func threadTitle(for threadID: String) -> String {
    TimelineSessionState.threadTitle(for: threadID, threads: threads)
  }

  func selectedEntry() -> TimelineEntry? {
    TimelineSessionState.selectedEntry(
      selectedEntryID: selectedEntryID,
      timeline: timeline
    )
  }

  func hasRuntimeThreadSelection(workspace: WorkspaceSummary?) -> Bool {
    TimelineSessionState.hasRuntimeThreadSelection(
      selectedThreadID: selectedThreadID,
      threads: threads,
      workspace: workspace
    )
  }

  func isWaitingForFirstMessage() -> Bool {
    TimelineSessionState.isWaitingForFirstMessage(
      selectedThreadID: selectedThreadID,
      threadTimelines: threadTimelines,
      visibleTimeline: timeline
    )
  }

  func hasAnnouncedSetupComplete(for threadID: String) -> Bool {
    announcedSetupCompleteThreadIDs.contains(threadID)
  }

  mutating func markSetupCompleteAnnounced(threadID: String) {
    announcedSetupCompleteThreadIDs.insert(threadID)
  }

  mutating func applyCreatedThread(_ thread: ThreadSummary) {
    threads.insert(thread, at: 0)
    threadTimelines[thread.id] = TimelineEntryFactory.defaultTimeline(for: thread.title)
    threadPendingApprovalIDs[thread.id] = Set<String>()
    selectedThreadID = thread.id
    syncVisibleTimeline()
  }

  mutating func applyWorkspaceThreads(_ workspaceThreads: [ThreadSummary]) {
    threads = workspaceThreads
    threadTimelines = TimelineMutationState.threadTimelines(
      for: workspaceThreads,
      existingTimelines: threadTimelines
    )
    threadPendingApprovalIDs = TimelineMutationState.pendingApprovalIDsByRetainingWorkspaceThreads(
      threadPendingApprovalIDs,
      workspaceThreads: workspaceThreads
    )
    selectedThreadID = TimelineMutationState.selectedThreadID(
      workspaceThreads: workspaceThreads,
      currentSelectionID: selectedThreadID
    )
    syncVisibleTimeline()
  }

  mutating func resetToWelcomeState(_ welcomeState: WelcomeTimelineState) {
    self = TimelineRuntimeState(welcomeState: welcomeState)
  }

  mutating func appendEntry(to threadID: String?, _ entry: TimelineEntry) {
    guard let threadID else {
      timeline.insert(entry, at: 0)
      if selectedEntryID == nil {
        selectedEntryID = entry.id
      }
      return
    }

    let entries = TimelineMutationState.entriesByAppending(
      entry: entry,
      existingEntries: threadTimelines[threadID],
      fallbackTitle: threadTitle(for: threadID)
    )
    applyThreadEntries(threadID: threadID, entries: entries)
  }

  mutating func applyThreadEntries(threadID: String, entries: [TimelineEntry]) {
    threadTimelines[threadID] = entries

    guard let visibleState = TimelineMutationState.visibleTimelineUpdate(
      updatedThreadID: threadID,
      selectedThreadID: selectedThreadID,
      entries: entries,
      previousSelectionID: selectedEntryID
    ) else {
      return
    }

    timeline = visibleState.timeline
    selectedEntryID = visibleState.selectedEntryID
  }

  mutating func syncVisibleTimeline() {
    let visibleState = TimelineMutationState.visibleTimeline(
      selectedThreadID: selectedThreadID,
      threadTimelines: threadTimelines,
      threads: threads,
      previousSelectionID: selectedEntryID
    )
    timeline = visibleState.timeline
    selectedEntryID = visibleState.selectedEntryID

    if let selectedThreadID {
      threadTimelines[selectedThreadID] = visibleState.timeline
    }
  }

  mutating func refreshThreadPreview(threadID: String, preview: String) {
    threads = TimelineMutationState.threadsByRefreshingPreview(
      threads,
      threadID: threadID,
      preview: preview
    )
  }

  mutating func updatePendingApprovals(
    threadID: String,
    approvals: [RuntimeBridge.RuntimeApproval]
  ) {
    threadPendingApprovalIDs[threadID] = TimelineMutationState.pendingApprovalIDs(from: approvals)
  }

  mutating func updateActiveTurn(threadID: String, activeTurnID: String?) {
    let activeTurnSelection = TimelineMutationState.activeTurnSelection(
      currentActiveTurnID: self.activeTurnID,
      currentActiveTurnThreadID: activeTurnThreadID,
      threadID: threadID,
      runtimeActiveTurnID: activeTurnID
    )
    self.activeTurnID = activeTurnSelection.activeTurnID
    activeTurnThreadID = activeTurnSelection.activeTurnThreadID
  }
}

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
      return ActiveTurnSelectionState(
        activeTurnID: nil,
        activeTurnThreadID: currentActiveTurnThreadID == threadID
          ? nil
          : currentActiveTurnThreadID
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
