import Foundation

@MainActor
extension AppViewModel {
  func createThread() {
    guard canCreateThread() else {
      return
    }

    guard let requestToken = threadCreationCoordinator.begin() else {
      return
    }
    let failureThreadID = selectedThreadID
    let title = "Thread \(threads.count + 1)"

    let task = Task {
      defer {
        threadCreationCoordinator.finish(requestToken)
      }
      do {
        let thread = try await runtimeBridge.startThread(title: title)
        guard threadCreationCoordinator.isCurrent(requestToken) else {
          return
        }
        await applyCreatedThread(thread)
        announceFirstRequestReadyIfNeeded()
      } catch {
        guard !Task.isCancelled,
              threadCreationCoordinator.isCurrent(requestToken)
        else {
          return
        }
        appendEntry(
          to: failureThreadID,
          TimelineEventPresenter.threadCreationFailed(error: error)
        )
      }
    }
    threadCreationCoordinator.bind(task: task, token: requestToken)
  }

  func selectThread(id: String?) {
    updateTimelineState { state in
      state.selectedThreadID = id
      state.syncVisibleTimeline()
    }

    guard runtimeState == .ready,
          let threadID = id,
          !threadID.hasPrefix("local-")
    else {
      return
    }

    Task {
      await loadThreadHistory(threadID: threadID)
      announceFirstRequestReadyIfNeeded()
    }
  }

  func refreshWorkspaceThreadSelection(
    from runtimeThreads: [RuntimeBridge.RuntimeThreadSummary],
    createIfEmpty: Bool
  ) async throws {
    guard let workspace else {
      resetToWelcomeThread()
      return
    }

    let workspaceThreads = try await WorkspaceThreadSelectionLoader.load(
      workspace: workspace,
      runtimeThreads: runtimeThreads,
      createIfEmpty: createIfEmpty,
      startThread: { [runtimeBridge] title in
        try await runtimeBridge.startThread(title: title)
      }
    )

    if workspaceThreads.isEmpty {
      resetToWelcomeThread()
      return
    }

    applyWorkspaceThreadSelection(workspaceThreads)
    if let selectedThreadID {
      await loadThreadHistory(threadID: selectedThreadID)
      announceFirstRequestReadyIfNeeded()
    }
  }

  func resetToWelcomeThread() {
    updateTimelineState { state in
      state.resetToWelcomeState(TimelineSessionState.welcomeState())
    }
  }

  func appendEntry(to threadID: String?, _ entry: TimelineEntry) {
    updateTimelineState { state in
      state.appendEntry(to: threadID, entry)
    }
  }

  func selectedEntry() -> TimelineEntry? {
    timelineState.selectedEntry()
  }

  func appendItemsToTimeline(
    threadID: String,
    items: [RuntimeBridge.RuntimeTimelineItemResult]
  ) {
    let newEntries = TimelineEntryFactory.transientEntries(from: items)

    for entry in newEntries.reversed() {
      appendEntry(to: threadID, entry)
    }
  }

  func updatePendingApprovals(
    threadID: String,
    approvals: [RuntimeBridge.RuntimeApproval]
  ) {
    updateTimelineState { state in
      state.updatePendingApprovals(threadID: threadID, approvals: approvals)
    }
  }

  func refreshThreadPreview(threadID: String, preview: String) {
    updateTimelineState { state in
      state.refreshThreadPreview(threadID: threadID, preview: preview)
    }
  }

  func loadThreadHistory(threadID: String) async {
    do {
      let result = try await runtimeBridge.readThread(threadID: threadID)
      let entries = TimelineEntryFactory.runtimeEntries(
        from: result.items,
        existingEntries: timelineState.threadTimelines[threadID],
        fallbackTitle: threadTitle(for: threadID)
      )
      applyThreadEntries(threadID: threadID, entries: entries)
      updatePendingApprovals(threadID: threadID, approvals: result.pendingApprovals)
      updateActiveTurn(threadID: threadID, activeTurnID: result.activeTurnID)
      refreshThreadPreview(threadID: threadID, preview: result.status)
    } catch {
      appendEntry(
        to: threadID,
        TimelineEventPresenter.threadLoadFailed(error: error)
      )
    }
  }

  func updateActiveTurn(threadID: String, activeTurnID: String?) {
    updateTimelineState { state in
      state.updateActiveTurn(threadID: threadID, activeTurnID: activeTurnID)
    }
  }

  func applyRuntimeThreadUpdate(_ state: RuntimeBridge.RuntimeThreadState) {
    let entries = TimelineEntryFactory.runtimeEntries(
      from: state.items,
      existingEntries: timelineState.threadTimelines[state.id],
      fallbackTitle: threadTitle(for: state.id)
    )

    applyThreadEntries(threadID: state.id, entries: entries)
    updatePendingApprovals(threadID: state.id, approvals: state.pendingApprovals)
    updateActiveTurn(threadID: state.id, activeTurnID: state.activeTurnID)
    refreshThreadPreview(threadID: state.id, preview: state.status)
  }

  private func applyCreatedThread(_ thread: ThreadSummary) async {
    updateTimelineState { state in
      state.applyCreatedThread(thread)
    }
    await loadThreadHistory(threadID: thread.id)
    appendEntry(
      to: thread.id,
      TimelineEventPresenter.threadCreated(thread)
    )
  }

  private func applyWorkspaceThreadSelection(_ workspaceThreads: [ThreadSummary]) {
    updateTimelineState { state in
      state.applyWorkspaceThreads(workspaceThreads)
    }
  }

  private func applyThreadEntries(threadID: String, entries: [TimelineEntry]) {
    updateTimelineState { state in
      state.applyThreadEntries(threadID: threadID, entries: entries)
    }
  }

  private func threadTitle(for threadID: String) -> String {
    timelineState.threadTitle(for: threadID)
  }
}

struct ThreadCreationRequestToken: Equatable {
  fileprivate let id: UUID
}

final class ThreadCreationCoordinator {
  private let taskSlot = CancellableTaskSlot()

  var isCreating: Bool {
    taskSlot.isActive
  }

  func begin() -> ThreadCreationRequestToken? {
    guard let requestID = taskSlot.begin() else {
      return nil
    }

    return ThreadCreationRequestToken(id: requestID)
  }

  func bind(task: Task<Void, Never>, token: ThreadCreationRequestToken) {
    taskSlot.bind(task: task, requestID: token.id)
  }

  func isCurrent(_ token: ThreadCreationRequestToken) -> Bool {
    taskSlot.isCurrent(token.id)
  }

  func finish(_ token: ThreadCreationRequestToken) {
    taskSlot.finish(token.id)
  }

  func cancel() {
    taskSlot.cancel()
  }
}
