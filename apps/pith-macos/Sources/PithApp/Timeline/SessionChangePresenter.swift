import Foundation

struct SessionRevertPrompt {
  let title: String
  let message: String
  let allowsRevert: Bool
  let confirmButtonTitle: String
}

struct SessionDeletePrompt {
  let title: String
  let message: String
  let confirmButtonTitle: String
}

enum SessionChangePresenter {
  static let activeWorkBlocksDeleteDetail = "Finish or cancel the current local work before deleting a session."
  static let activeWorkBlocksRevertDetail = "Finish or cancel the current local work before reverting session changes."
  static let noRevertableChangesDetail = "This session has not saved any workspace files."
  static let deleteSuccessDetail = "Deleted session. Workspace files were not changed."

  static func deleteFailedDetail(error: Error) -> String {
    "Session delete failed: \(error.localizedDescription)"
  }

  static func revertPreviewFailedDetail(error: Error) -> String {
    "Could not review session changes: \(error.localizedDescription)"
  }

  static func revertFailedDetail(error: Error) -> String {
    "Session revert failed: \(error.localizedDescription)"
  }

  static func revertSuccessDetail(revertedCount: Int) -> String {
    switch revertedCount {
    case 1:
      return "Reverted 1 file saved by this session."
    default:
      return "Reverted \(revertedCount) files saved by this session."
    }
  }

  static func deletePrompt() -> SessionDeletePrompt {
    SessionDeletePrompt(
      title: "Delete Session?",
      message: """
      Pith will delete this session's messages, timeline, and pending approvals.

      Workspace files and repositories will not be deleted or reverted.
      If you want to undo files Pith saved, use Review Session Changes before deleting the session.
      """,
      confirmButtonTitle: "Delete Session"
    )
  }

  static func revertPrompt(for preview: RuntimeBridge.RuntimeThreadChangePreview) -> SessionRevertPrompt {
    let hasConflicts = preview.changes.contains { !$0.canRevert }
    let title = hasConflicts ? "Review Session Changes" : "Revert Session Changes?"
    let actionLine = hasConflicts
      ? "Some files changed after Pith saved them, so Pith will leave everything untouched for now."
      : "Pith will only revert files that still match what it saved."

    return SessionRevertPrompt(
      title: title,
      message: """
      \(countLine(preview.changes.count))

      \(changeList(preview.changes))

      \(actionLine) The session itself will stay.
      """,
      allowsRevert: !hasConflicts,
      confirmButtonTitle: "Revert Changes"
    )
  }

  private static func countLine(_ count: Int) -> String {
    switch count {
    case 1:
      return "Pith can review 1 file saved by this session."
    default:
      return "Pith can review \(count) files saved by this session."
    }
  }

  private static func changeList(_ changes: [RuntimeBridge.RuntimeThreadChange]) -> String {
    let visibleChanges = changes
      .prefix(5)
      .map(changeLine)
      .joined(separator: "\n")
    let hiddenCount = max(0, changes.count - 5)
    let hiddenSuffix = hiddenCount > 0 ? "\n- and \(hiddenCount) more" : ""
    return "\(visibleChanges)\(hiddenSuffix)"
  }

  private static func changeLine(_ change: RuntimeBridge.RuntimeThreadChange) -> String {
    guard let conflictReason = change.conflictReason else {
      return "- \(change.relativePath)"
    }

    return "- \(change.relativePath): \(conflictDetail(conflictReason))"
  }

  private static func conflictDetail(_ reason: String) -> String {
    if reason.contains("changed after Pith wrote it") {
      return "changed after Pith saved it"
    }
    if reason.contains("failed to read current file content") {
      return "is no longer available"
    }
    return "needs review before Pith can revert it"
  }
}
