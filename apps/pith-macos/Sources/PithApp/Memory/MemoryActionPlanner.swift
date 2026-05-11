import Foundation

struct MemoryActionSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let hasWorkspace: Bool
  let isSavingNote: Bool
  let title: String
  let body: String
}

struct PreparedMemoryNoteDraft {
  let title: String
  let body: String
}

enum MemoryActionPlanner {
  static func preparedDraft(_ snapshot: MemoryActionSnapshot) -> PreparedMemoryNoteDraft? {
    let title = snapshot.title.trimmingCharacters(in: .whitespacesAndNewlines)
    let body = snapshot.body.trimmingCharacters(in: .whitespacesAndNewlines)

    guard snapshot.runtimeState == .ready,
          snapshot.hasWorkspace,
          !snapshot.isSavingNote,
          !title.isEmpty,
          !body.isEmpty
    else {
      return nil
    }

    return PreparedMemoryNoteDraft(title: title, body: body)
  }

  static func canSave(_ snapshot: MemoryActionSnapshot) -> Bool {
    preparedDraft(snapshot) != nil
  }
}
