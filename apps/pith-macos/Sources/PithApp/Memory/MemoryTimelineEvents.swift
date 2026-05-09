import Foundation

extension TimelineEventPresenter {
  static func memoryNoteSaved(_ note: RuntimeBridge.RuntimeMemoryNote) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Memory Note Saved",
      body: "Saved built-in workspace note \(note.title).",
      attributes: [
        "memoryNoteId": note.id,
        "memoryScope": note.scope,
        "memorySource": note.source,
      ]
    )
  }

  static func memoryNoteFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Memory Note Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }
}
