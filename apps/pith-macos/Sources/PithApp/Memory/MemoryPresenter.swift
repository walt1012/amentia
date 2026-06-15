import Foundation

struct MemorySnapshot {
  let status: MemoryStatusSummary?
  let notes: [MemoryNoteSummary]
}

enum MemoryPresenter {
  static func countSummary(_ snapshot: MemorySnapshot) -> String {
    guard let status = snapshot.status else {
      return "Built-in memory is not connected yet."
    }

    return "\(status.noteCount) note(s) captured"
  }

  static func detailSummary(_ snapshot: MemorySnapshot) -> String {
    guard let status = snapshot.status else {
      return "Pith uses built-in local memory. Project notes stay on this Mac."
    }

    if snapshot.notes.isEmpty {
      return status.summary
    }

    return snapshot.notes
      .prefix(4)
      .map { note in
        let tagSummary = note.tags.isEmpty ? "untagged" : note.tags.joined(separator: ", ")
        return "\(note.title) | \(note.scope) | \(note.source) | tags: \(tagSummary)"
      }
      .joined(separator: "\n")
  }

  static func latestSummary(_ snapshot: MemorySnapshot) -> String {
    guard let latestNote = snapshot.notes.first else {
      return "No memory notes have been captured yet."
    }

    return "\(latestNote.body)\nSource: \(latestNote.source)"
  }
}
