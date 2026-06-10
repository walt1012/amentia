import Foundation

struct SessionOverviewSnapshot {
  let selectedThread: ThreadSummary?
  let workspace: WorkspaceSummary?
  let selectedEntry: TimelineEntry?
}

enum SessionOverviewPresenter {
  static func selectedThreadTitle(_ snapshot: SessionOverviewSnapshot) -> String {
    snapshot.selectedThread?.title ?? "No Session Selected"
  }

  static func selectedThreadPreview(_ snapshot: SessionOverviewSnapshot) -> String {
    snapshot.selectedThread?.preview ?? "Select a thread to inspect its runtime state."
  }

  static func workspaceDisplayName(_ snapshot: SessionOverviewSnapshot) -> String {
    snapshot.workspace?.displayName ?? "No Workspace"
  }

  static func workspacePath(_ snapshot: SessionOverviewSnapshot) -> String {
    snapshot.workspace?.rootPath ?? "Open a local workspace to enable project-scoped tools."
  }

  static func shouldShowSelectedEntryInspector(_ snapshot: SessionOverviewSnapshot) -> Bool {
    guard let entry = snapshot.selectedEntry else {
      return false
    }

    if entry.kind != .system {
      return true
    }

    return !entry.id.hasPrefix("welcome-")
      && !entry.id.hasPrefix("default-thread-ready:")
  }
}
