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
  static let activeWorkBlocksDeleteDetail = "Finish or cancel the current work before deleting a session."
  static let activeWorkBlocksRevertDetail = "Finish or cancel the current work before reverting session changes."
  static let sessionOperationInProgressDetail =
    "Finish the current session operation before starting another one."
  static let noRevertableChangesDetail = "This session has not saved any project files."
  static let deleteReceiptTitle = "Session Deleted"

  static func deletingDetail(threadTitle: String) -> String {
    "Deleting \(threadTitle)..."
  }

  static func revertingDetail(threadTitle: String) -> String {
    "Reverting changes saved by \(threadTitle)..."
  }

  static func deleteSuccessDetail(threadTitle: String) -> String {
    "Deleted \(threadTitle). Project files were not changed."
  }

  static func deleteReceiptBody(threadTitle: String) -> String {
    "\(threadTitle) was removed from Amentia. Project files and repositories were not changed."
  }

  static func deleteFailedDetail(error _: Error) -> String {
    "Could not delete the session. Try again after Amentia finishes syncing local state."
  }

  static func revertPreviewFailedDetail(error _: Error) -> String {
    "Could not review session changes. Check that the project is still available, then try again."
  }

  static func revertFailedDetail(error _: Error) -> String {
    "Could not revert session changes. Review the session changes again before retrying."
  }

  static func revertSuccessDetail(revertedCount: Int) -> String {
    switch revertedCount {
    case 0:
      return "No files were reverted. Project files were not changed."
    case 1:
      return "Reverted 1 file saved by this session."
    default:
      return "Reverted \(revertedCount) files saved by this session."
    }
  }

  static func revertThreadPreview(revertedCount: Int) -> String {
    switch revertedCount {
    case 0:
      return "No session changes to revert"
    case 1:
      return "Reverted 1 file"
    default:
      return "Reverted \(revertedCount) files"
    }
  }

  static func deletePrompt(threadTitle: String? = nil) -> SessionDeletePrompt {
    let titleLine = threadTitle
      .map { "Amentia will delete \"\($0)\" from the session list." }
      ?? "Amentia will delete this session from the session list."

    return SessionDeletePrompt(
      title: "Delete Session?",
      message: """
      \(titleLine)

      This removes chat history, activity cards, and unfinished permission requests for that session.

      Project files and repositories will not be deleted or reverted.
      If you want to undo files Amentia saved, use Review Session Changes before deleting the session.
      """,
      confirmButtonTitle: "Delete Session"
    )
  }

  static func revertPrompt(
    for preview: RuntimeBridge.RuntimeThreadChangePreview,
    threadTitle: String? = nil
  ) -> SessionRevertPrompt {
    let hasConflicts = preview.changes.contains { !$0.canRevert }
    let title = hasConflicts ? "Review Session Changes" : "Revert Session Changes?"
    let titleLine = threadTitle
      .map { "Amentia will review changes saved by \"\($0)\"." }
      ?? "Amentia will review changes saved by this session."
    let actionLine = hasConflicts
      ? "Some files changed after Amentia saved them, so Amentia will leave everything untouched for now."
      : "Amentia will only revert files that still match what it saved."

    return SessionRevertPrompt(
      title: title,
      message: """
      \(titleLine)

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
      return "Amentia can review 1 file saved by this session."
    default:
      return "Amentia can review \(count) files saved by this session."
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
    if reason.contains("changed after Amentia wrote it") || reason.contains("changed after Amentia saved it") {
      return "changed after Amentia saved it"
    }
    if reason.contains("failed to read current file content") {
      return "is no longer available"
    }
    return "needs review before Amentia can revert it"
  }
}
