import Foundation

@MainActor
extension AppViewModel {
  func saveWorkspaceMemoryNote() {
    guard let draft = MemoryActionPlanner.preparedDraft(memoryActionSnapshot()) else {
      return
    }

    Task {
      do {
        let note = try await runtimeBridge.createMemoryNote(title: draft.title, body: draft.body)
        updateMemoryState { state in
          state.clearDraft()
        }
        await refreshMemoryState()
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.memoryNoteSaved(note)
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
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
      title: memoryNoteTitle,
      body: memoryNoteBody
    )
  }
}
