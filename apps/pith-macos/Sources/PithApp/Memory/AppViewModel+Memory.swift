import Foundation

@MainActor
extension AppViewModel {
  func saveWorkspaceMemoryNote() {
    guard let draft = MemoryActionPlanner.preparedDraft(memoryActionSnapshot()) else {
      return
    }

    guard let operationID = beginMemorySaveOperation() else {
      return
    }
    let timelineThreadID = selectedThreadID
    runtimeDetail = "Saving memory note..."

    Task {
      defer {
        finishMemorySaveOperation(operationID)
      }
      do {
        let note = try await runtimeBridge.createMemoryNote(title: draft.title, body: draft.body)
        updateMemoryState { state in
          state.clearDraft()
        }
        await refreshMemoryState()
        appendEntry(
          to: timelineThreadID,
          TimelineEventPresenter.memoryNoteSaved(note)
        )
      } catch {
        appendEntry(
          to: timelineThreadID,
          TimelineEventPresenter.memoryNoteFailed(error: error)
        )
      }
    }
  }

  func memoryCountSummary() -> String {
    MemoryPresenter.countSummary(memorySnapshot())
  }

  func memoryDetailSummary() -> String {
    MemoryPresenter.detailSummary(memorySnapshot())
  }

  func memoryLatestSummary() -> String {
    MemoryPresenter.latestSummary(memorySnapshot())
  }

  func canSaveWorkspaceMemoryNote() -> Bool {
    MemoryActionPlanner.canSave(memoryActionSnapshot())
  }

  func refreshMemoryState() async {
    let memoryRefresh = await MemoryStateLoader.refresh(using: runtimeBridge)
    applyMemoryStateRefresh(memoryRefresh, clearsMissing: false)
  }

  func applyMemoryStateRefresh(
    _ memoryRefresh: MemoryStateRefresh,
    clearsMissing: Bool
  ) {
    updateMemoryState { state in
      state.apply(memoryRefresh, clearsMissing: clearsMissing)
    }
  }

  private func memorySnapshot() -> MemorySnapshot {
    MemorySnapshot(
      status: memoryStatus,
      notes: memoryNotes
    )
  }

  private func memoryActionSnapshot() -> MemoryActionSnapshot {
    MemoryActionSnapshot(
      runtimeState: runtimeState,
      hasWorkspace: workspace != nil,
      isSavingNote: isSavingMemoryNote,
      title: memoryNoteTitle,
      body: memoryNoteBody
    )
  }

  private func beginMemorySaveOperation() -> UUID? {
    var operationID: UUID?
    updateMemoryState { state in
      operationID = state.beginSaveOperation()
    }
    return operationID
  }

  private func finishMemorySaveOperation(_ operationID: UUID) {
    updateMemoryState { state in
      state.finishSaveOperation(operationID)
    }
  }
}
