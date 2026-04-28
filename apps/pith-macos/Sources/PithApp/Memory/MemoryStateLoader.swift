import Foundation

struct MemoryStateRefresh {
  let status: MemoryStatusSummary?
  let notes: [MemoryNoteSummary]?
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
