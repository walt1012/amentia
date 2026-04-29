import Foundation

struct MemoryStateRefresh {
  let status: MemoryStatusSummary?
  let notes: [MemoryNoteSummary]?
}

struct MemoryRuntimeState {
  var status: MemoryStatusSummary?
  var notes: [MemoryNoteSummary]
  var noteTitle: String
  var noteBody: String

  init(
    status: MemoryStatusSummary? = nil,
    notes: [MemoryNoteSummary] = [],
    noteTitle: String = "",
    noteBody: String = ""
  ) {
    self.status = status
    self.notes = notes
    self.noteTitle = noteTitle
    self.noteBody = noteBody
  }

  mutating func apply(_ refresh: MemoryStateRefresh, clearsMissing: Bool) {
    if clearsMissing || refresh.status != nil {
      status = refresh.status
    }
    if clearsMissing || refresh.notes != nil {
      notes = refresh.notes ?? []
    }
  }

  mutating func clearDraft() {
    noteTitle = ""
    noteBody = ""
  }

  mutating func resetRuntimeData() {
    status = nil
    notes = []
  }
}

enum MemoryStateLoader {
  static func refresh(using runtimeBridge: RuntimeBridge) async -> MemoryStateRefresh {
    let runtimeMemoryStatus = try? await runtimeBridge.memoryStatus()
    let runtimeMemoryNotes = try? await runtimeBridge.listMemoryNotes()

    return MemoryStateRefresh(
      status: runtimeMemoryStatus.map { RuntimeSummaryMapper.memoryStatusSummary(from: $0) },
      notes: runtimeMemoryNotes.map { notes in
        notes.map { RuntimeSummaryMapper.memoryNoteSummary(from: $0) }
      }
    )
  }
}
