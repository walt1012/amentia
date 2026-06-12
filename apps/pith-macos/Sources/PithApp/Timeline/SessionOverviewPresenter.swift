import Foundation

struct SessionOverviewSnapshot {
  let selectedThread: ThreadSummary?
  let workspace: WorkspaceSummary?
  let selectedEntry: TimelineEntry?
}

struct SessionPreviewSnapshot {
  let status: String
  let workspaceDisplayName: String?
  let pendingApprovalCount: Int
  let hasActiveTurn: Bool
}

enum SessionOverviewPresenter {
  static func selectedThreadTitle(_ snapshot: SessionOverviewSnapshot) -> String {
    snapshot.selectedThread?.title ?? "No Session Selected"
  }

  static func selectedThreadPreview(_ snapshot: SessionOverviewSnapshot) -> String {
    snapshot.selectedThread?.preview ?? "Select a session to inspect its current state."
  }

  static func runtimeThreadPreview(
    status: String,
    workspaceDisplayName: String? = nil,
    pendingApprovalCount: Int = 0,
    hasActiveTurn: Bool = false
  ) -> String {
    runtimeThreadPreview(SessionPreviewSnapshot(
      status: status,
      workspaceDisplayName: workspaceDisplayName,
      pendingApprovalCount: pendingApprovalCount,
      hasActiveTurn: hasActiveTurn
    ))
  }

  static func runtimeThreadPreview(_ snapshot: SessionPreviewSnapshot) -> String {
    if snapshot.hasActiveTurn {
      return workspaceScopedPreview(
        "Working",
        workspaceDisplayName: snapshot.workspaceDisplayName
      )
    }

    if snapshot.pendingApprovalCount == 1 {
      return "Waiting for your approval."
    }

    if snapshot.pendingApprovalCount > 1 {
      return "Waiting for \(snapshot.pendingApprovalCount) approvals."
    }

    switch snapshot.status.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
    case "ready":
      return workspaceScopedPreview(
        "Ready to continue",
        workspaceDisplayName: snapshot.workspaceDisplayName
      )
    case "running", "active", "busy":
      return workspaceScopedPreview(
        "Working",
        workspaceDisplayName: snapshot.workspaceDisplayName
      )
    case "cancelled":
      return "Last request was cancelled."
    case "failed", "error":
      return "Needs attention before continuing."
    case "approval", "needs_approval", "needs-approval":
      return "Waiting for your approval."
    case "":
      return workspaceScopedPreview(
        "Ready to continue",
        workspaceDisplayName: snapshot.workspaceDisplayName
      )
    default:
      return workspaceScopedPreview(
        "Ready to continue",
        workspaceDisplayName: snapshot.workspaceDisplayName
      )
    }
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

  private static func workspaceScopedPreview(
    _ summary: String,
    workspaceDisplayName: String?
  ) -> String {
    guard let workspaceDisplayName,
          !workspaceDisplayName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    else {
      return "\(summary)."
    }

    return "\(summary) in \(workspaceDisplayName)."
  }
}
