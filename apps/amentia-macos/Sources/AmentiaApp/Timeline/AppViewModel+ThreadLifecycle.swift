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
    let title = "Session \(threads.count + 1)"

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
      threadHistoryLoadCoordinator.cancel()
      return
    }

    let requestToken = threadHistoryLoadCoordinator.begin(threadID: threadID)
    let task = Task {
      defer {
        threadHistoryLoadCoordinator.finish(requestToken)
      }
      do {
        let result = try await runtimeBridge.readThread(threadID: requestToken.threadID)
        guard threadHistoryLoadCoordinator.isCurrent(requestToken) else {
          return
        }
        applyLoadedThreadHistory(result)
        announceFirstRequestReadyIfNeeded()
      } catch {
        guard !Task.isCancelled,
              threadHistoryLoadCoordinator.isCurrent(requestToken)
        else {
          return
        }
        appendEntry(
          to: requestToken.threadID,
          TimelineEventPresenter.threadLoadFailed(error: error)
        )
      }
    }
    threadHistoryLoadCoordinator.bind(task: task, token: requestToken)
  }

  func canDeleteThread(_ thread: ThreadSummary) -> Bool {
    runtimeState == .ready
      && !thread.id.hasPrefix("local-")
      && !hasActiveOrPendingTurn()
      && !threadMutationCoordinator.isMutating
  }

  func canRevertThreadChanges(_ thread: ThreadSummary) -> Bool {
    runtimeState == .ready
      && !thread.id.hasPrefix("local-")
      && !hasActiveOrPendingTurn()
      && !threadMutationCoordinator.isMutating
  }

  func previewThreadChanges(
    _ thread: ThreadSummary
  ) async -> RuntimeBridge.RuntimeThreadChangePreview? {
    guard canRevertThreadChanges(thread) else {
      runtimeDetail = threadMutationCoordinator.isMutating
        ? SessionChangePresenter.sessionOperationInProgressDetail
        : SessionChangePresenter.activeWorkBlocksRevertDetail
      return nil
    }

    do {
      let preview = try await runtimeBridge.previewThreadChanges(threadID: thread.id)
      guard !Task.isCancelled else {
        return nil
      }
      if preview.changes.isEmpty {
        runtimeDetail = SessionChangePresenter.noRevertableChangesDetail
        return nil
      }
      return preview
    } catch {
      guard !Task.isCancelled else {
        return nil
      }
      runtimeDetail = SessionChangePresenter.revertPreviewFailedDetail(error: error)
      return nil
    }
  }

  func revertThreadChanges(_ thread: ThreadSummary) {
    guard runtimeState == .ready,
          !thread.id.hasPrefix("local-")
    else {
      return
    }
    guard !hasActiveOrPendingTurn() else {
      runtimeDetail = SessionChangePresenter.activeWorkBlocksRevertDetail
      return
    }
    guard let requestToken = threadMutationCoordinator.begin(threadID: thread.id, kind: .revert)
    else {
      runtimeDetail = SessionChangePresenter.sessionOperationInProgressDetail
      return
    }

    runtimeDetail = SessionChangePresenter.revertingDetail(threadTitle: thread.title)
    let task = Task {
      defer {
        threadMutationCoordinator.finish(requestToken)
      }
      do {
        let result = try await runtimeBridge.revertThreadChanges(threadID: thread.id)
        guard !Task.isCancelled,
              threadMutationCoordinator.isCurrent(requestToken)
        else {
          return
        }
        appendItemsToTimeline(threadID: result.threadID, items: result.items)
        refreshThreadPreview(
          threadID: result.threadID,
          preview: SessionChangePresenter.revertThreadPreview(revertedCount: result.revertedCount)
        )
        runtimeDetail = SessionChangePresenter.revertSuccessDetail(revertedCount: result.revertedCount)
      } catch {
        guard !Task.isCancelled,
              threadMutationCoordinator.isCurrent(requestToken)
        else {
          return
        }
        runtimeDetail = SessionChangePresenter.revertFailedDetail(error: error)
      }
    }
    threadMutationCoordinator.bind(task: task, token: requestToken)
  }

  func deleteThread(_ thread: ThreadSummary) {
    guard runtimeState == .ready,
          !thread.id.hasPrefix("local-")
    else {
      return
    }
    guard !hasActiveOrPendingTurn() else {
      runtimeDetail = SessionChangePresenter.activeWorkBlocksDeleteDetail
      return
    }
    guard let requestToken = threadMutationCoordinator.begin(threadID: thread.id, kind: .delete)
    else {
      runtimeDetail = SessionChangePresenter.sessionOperationInProgressDetail
      return
    }

    let deletedThreadID = thread.id
    let deletedThreadTitle = thread.title
    runtimeDetail = SessionChangePresenter.deletingDetail(threadTitle: deletedThreadTitle)
    let task = Task {
      defer {
        threadMutationCoordinator.finish(requestToken)
      }
      do {
        let runtimeThreads = try await runtimeBridge.deleteThread(threadID: deletedThreadID)
        guard !Task.isCancelled,
              threadMutationCoordinator.isCurrent(requestToken)
        else {
          return
        }
        await applyDeletedThread(
          threadID: deletedThreadID,
          threadTitle: deletedThreadTitle,
          runtimeThreads: runtimeThreads
        )
      } catch {
        guard !Task.isCancelled,
              threadMutationCoordinator.isCurrent(requestToken)
        else {
          return
        }
        runtimeDetail = SessionChangePresenter.deleteFailedDetail(error: error)
      }
    }
    threadMutationCoordinator.bind(task: task, token: requestToken)
    threadHistoryLoadCoordinator.cancel()
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
      applyLoadedThreadHistory(result)
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
    refreshThreadPreview(threadID: state.id, preview: runtimeThreadPreview(for: state))
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

  private func applyLoadedThreadHistory(_ result: RuntimeBridge.RuntimeThreadState) {
    let entries = TimelineEntryFactory.runtimeEntries(
      from: result.items,
      existingEntries: timelineState.threadTimelines[result.id],
      fallbackTitle: threadTitle(for: result.id)
    )
    applyThreadEntries(threadID: result.id, entries: entries)
    updatePendingApprovals(threadID: result.id, approvals: result.pendingApprovals)
    updateActiveTurn(threadID: result.id, activeTurnID: result.activeTurnID)
    refreshThreadPreview(threadID: result.id, preview: runtimeThreadPreview(for: result))
  }

  private func applyWorkspaceThreadSelection(_ workspaceThreads: [ThreadSummary]) {
    updateTimelineState { state in
      state.applyWorkspaceThreads(workspaceThreads)
    }
  }

  private func applyDeletedThread(
    threadID: String,
    threadTitle: String,
    runtimeThreads: [RuntimeBridge.RuntimeThreadSummary]
  ) async {
    let workspaceThreads: [ThreadSummary]
    if let workspace {
      workspaceThreads = (try? await WorkspaceThreadSelectionLoader.load(
        workspace: workspace,
        runtimeThreads: runtimeThreads,
        createIfEmpty: false,
        startThread: { [runtimeBridge] title in
          try await runtimeBridge.startThread(title: title)
        }
      )) ?? []
    } else {
      workspaceThreads = runtimeThreads.map(RuntimeSummaryMapper.threadSummary(from:))
    }

    updateTimelineState { state in
      state.deleteThread(threadID: threadID, remainingThreads: workspaceThreads)
    }
    appendEntry(
      to: selectedThreadID,
      TimelineEntryFactory.system(
        title: SessionChangePresenter.deleteReceiptTitle,
        body: SessionChangePresenter.deleteReceiptBody(threadTitle: threadTitle),
        attributes: [
          "action": "thread.delete",
          "deletedThreadID": threadID,
        ]
      )
    )
    runtimeDetail = SessionChangePresenter.deleteSuccessDetail(threadTitle: threadTitle)
    announceFirstRequestReadyIfNeeded()
  }

  private func applyThreadEntries(threadID: String, entries: [TimelineEntry]) {
    updateTimelineState { state in
      state.applyThreadEntries(threadID: threadID, entries: entries)
    }
  }

  private func threadTitle(for threadID: String) -> String {
    timelineState.threadTitle(for: threadID)
  }

  private func runtimeThreadPreview(for state: RuntimeBridge.RuntimeThreadState) -> String {
    SessionOverviewPresenter.runtimeThreadPreview(
      status: state.status,
      workspaceDisplayName: threads.first(where: { $0.id == state.id })?.workspaceDisplayName
        ?? workspace?.displayName,
      pendingApprovalCount: state.pendingApprovals.count,
      hasActiveTurn: state.activeTurnID != nil
    )
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

struct ThreadHistoryLoadRequestToken: Equatable {
  fileprivate let id: UUID
  let threadID: String
}

final class ThreadHistoryLoadCoordinator {
  private let taskSlot = CancellableTaskSlot()

  func begin(threadID: String) -> ThreadHistoryLoadRequestToken {
    ThreadHistoryLoadRequestToken(
      id: taskSlot.replace(),
      threadID: threadID
    )
  }

  func bind(task: Task<Void, Never>, token: ThreadHistoryLoadRequestToken) {
    taskSlot.bind(task: task, requestID: token.id)
  }

  func isCurrent(_ token: ThreadHistoryLoadRequestToken) -> Bool {
    taskSlot.isCurrent(token.id)
  }

  func finish(_ token: ThreadHistoryLoadRequestToken) {
    taskSlot.finish(token.id)
  }

  func cancel() {
    taskSlot.cancel()
  }
}

enum ThreadMutationKind: Equatable {
  case delete
  case revert
}

struct ThreadMutationRequestToken: Equatable {
  fileprivate let id: UUID
  let threadID: String
  let kind: ThreadMutationKind
}

final class ThreadMutationCoordinator {
  private let taskSlot = CancellableTaskSlot()
  private(set) var activeThreadID: String?
  private(set) var activeKind: ThreadMutationKind?

  var isMutating: Bool {
    taskSlot.isActive
  }

  func begin(threadID: String, kind: ThreadMutationKind) -> ThreadMutationRequestToken? {
    guard let requestID = taskSlot.begin() else {
      return nil
    }

    activeThreadID = threadID
    activeKind = kind
    return ThreadMutationRequestToken(
      id: requestID,
      threadID: threadID,
      kind: kind
    )
  }

  func bind(task: Task<Void, Never>, token: ThreadMutationRequestToken) {
    taskSlot.bind(task: task, requestID: token.id)
  }

  func isCurrent(_ token: ThreadMutationRequestToken) -> Bool {
    taskSlot.isCurrent(token.id)
  }

  func finish(_ token: ThreadMutationRequestToken) {
    guard isCurrent(token) else {
      return
    }

    activeThreadID = nil
    activeKind = nil
    taskSlot.finish(token.id)
  }

  func cancel() {
    activeThreadID = nil
    activeKind = nil
    taskSlot.cancel()
  }
}
