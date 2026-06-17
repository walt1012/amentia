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

  mutating func deleteThread(threadID: String, remainingThreads: [ThreadSummary]) {
    threads = remainingThreads
    threadTimelines.removeValue(forKey: threadID)
    threadPendingApprovalIDs.removeValue(forKey: threadID)
    announcedSetupCompleteThreadIDs.remove(threadID)
    if activeTurnThreadID == threadID {
      activeTurnID = nil
      activeTurnThreadID = nil
    }
    selectedThreadID = TimelineMutationState.selectedThreadID(
      workspaceThreads: remainingThreads,
      currentSelectionID: selectedThreadID == threadID ? nil : selectedThreadID
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
