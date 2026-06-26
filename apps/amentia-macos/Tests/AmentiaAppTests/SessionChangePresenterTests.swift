@testable import AmentiaApp
import XCTest

final class SessionChangePresenterTests: XCTestCase {
  func testDeletePromptSeparatesSessionRemovalFromWorkspaceFiles() {
    let prompt = SessionChangePresenter.deletePrompt(threadTitle: "Design Review")

    XCTAssertEqual(prompt.title, "Delete Session?")
    XCTAssertEqual(prompt.confirmButtonTitle, "Delete Session")
    XCTAssertTrue(prompt.message.contains("delete \"Design Review\" from the session list"))
    XCTAssertTrue(prompt.message.contains("chat history, activity cards, and unfinished permission requests"))
    XCTAssertTrue(prompt.message.contains("Project files and repositories will not be deleted"))
    XCTAssertTrue(prompt.message.contains("use Review Session Changes before deleting"))
    XCTAssertFalse(prompt.message.contains("timeline"))
    XCTAssertFalse(prompt.message.contains("approval"))
  }

  func testDeleteSuccessCopyNamesSessionAndPreservesProjectFiles() {
    XCTAssertEqual(
      SessionChangePresenter.deletingDetail(threadTitle: "Design Review"),
      "Deleting Design Review..."
    )
    XCTAssertEqual(
      SessionChangePresenter.deleteSuccessDetail(threadTitle: "Design Review"),
      "Deleted Design Review. Project files were not changed."
    )
    XCTAssertEqual(SessionChangePresenter.deleteReceiptTitle, "Session Deleted")
    XCTAssertEqual(
      SessionChangePresenter.deleteReceiptBody(threadTitle: "Design Review"),
      "Design Review was removed from Amentia. Project files and repositories were not changed."
    )
  }

  func testSessionChangeFailuresAvoidInternalErrors() {
    let error = NSError(domain: "AmentiaTests", code: 1, userInfo: [
      NSLocalizedDescriptionKey: "JSON-RPC thread/delete failed for /tmp/project"
    ])

    let delete = SessionChangePresenter.deleteFailedDetail(error: error)
    let preview = SessionChangePresenter.revertPreviewFailedDetail(error: error)
    let revert = SessionChangePresenter.revertFailedDetail(error: error)

    XCTAssertTrue(delete.contains("Could not delete the session"))
    XCTAssertTrue(preview.contains("project is still available"))
    XCTAssertTrue(revert.contains("Review the session changes again"))

    for detail in [delete, preview, revert] {
      XCTAssertFalse(detail.contains("JSON-RPC"))
      XCTAssertFalse(detail.contains("thread/delete"))
      XCTAssertFalse(detail.contains("/tmp/project"))
    }
  }

  func testSessionOperationInProgressCopyIsUserFacing() {
    XCTAssertEqual(
      SessionChangePresenter.sessionOperationInProgressDetail,
      "Finish the current session operation before starting another one."
    )
    XCTAssertEqual(
      SessionChangePresenter.revertingDetail(threadTitle: "Design Review"),
      "Reverting changes saved by Design Review..."
    )
    XCTAssertFalse(SessionChangePresenter.sessionOperationInProgressDetail.contains("thread"))
  }

  func testRevertSuccessCopyHandlesZeroOneAndManyFiles() {
    XCTAssertEqual(
      SessionChangePresenter.revertSuccessDetail(revertedCount: 0),
      "No files were reverted. Project files were not changed."
    )
    XCTAssertEqual(
      SessionChangePresenter.revertThreadPreview(revertedCount: 0),
      "No session changes to revert"
    )
    XCTAssertEqual(
      SessionChangePresenter.revertSuccessDetail(revertedCount: 1),
      "Reverted 1 file saved by this session."
    )
    XCTAssertEqual(
      SessionChangePresenter.revertThreadPreview(revertedCount: 1),
      "Reverted 1 file"
    )
    XCTAssertEqual(
      SessionChangePresenter.revertSuccessDetail(revertedCount: 3),
      "Reverted 3 files saved by this session."
    )
    XCTAssertEqual(
      SessionChangePresenter.revertThreadPreview(revertedCount: 3),
      "Reverted 3 files"
    )
  }

  func testRevertPromptAllowsCleanRecordedChanges() {
    let prompt = SessionChangePresenter.revertPrompt(
      for: preview(changes: [
        change(path: "Sources/App.swift"),
        change(path: "README.md"),
      ]),
      threadTitle: "Design Review"
    )

    XCTAssertEqual(prompt.title, "Revert Session Changes?")
    XCTAssertEqual(prompt.confirmButtonTitle, "Revert Changes")
    XCTAssertTrue(prompt.allowsRevert)
    XCTAssertTrue(prompt.message.contains("changes saved by \"Design Review\""))
    XCTAssertTrue(prompt.message.contains("Amentia can review 2 files"))
    XCTAssertTrue(prompt.message.contains("- Sources/App.swift"))
    XCTAssertTrue(prompt.message.contains("The session itself will stay"))
  }

  func testRevertPromptBlocksWhenFilesChangedAfterAmentiaSavedThem() {
    let prompt = SessionChangePresenter.revertPrompt(for: preview(changes: [
      change(
        path: "Sources/App.swift",
        canRevert: false,
        conflictReason: "changed after Amentia wrote it"
      ),
    ]))

    XCTAssertEqual(prompt.title, "Review Session Changes")
    XCTAssertFalse(prompt.allowsRevert)
    XCTAssertTrue(prompt.message.contains("changed after Amentia saved it"))
    XCTAssertTrue(prompt.message.contains("leave everything untouched for now"))
  }

  func testRevertPromptAcceptsSavedConflictWording() {
    let prompt = SessionChangePresenter.revertPrompt(for: preview(changes: [
      change(
        path: "Sources/App.swift",
        canRevert: false,
        conflictReason: "changed after Amentia saved it"
      ),
    ]))

    XCTAssertTrue(prompt.message.contains("changed after Amentia saved it"))
  }

  func testRevertPromptKeepsLongChangeListsReadable() {
    let prompt = SessionChangePresenter.revertPrompt(for: preview(changes: [
      change(path: "one.swift"),
      change(path: "two.swift"),
      change(path: "three.swift"),
      change(path: "four.swift"),
      change(path: "five.swift"),
      change(path: "six.swift"),
    ]))

    XCTAssertTrue(prompt.message.contains("- one.swift"))
    XCTAssertTrue(prompt.message.contains("- five.swift"))
    XCTAssertFalse(prompt.message.contains("- six.swift"))
    XCTAssertTrue(prompt.message.contains("- and 1 more"))
  }

  private func preview(
    changes: [RuntimeBridge.RuntimeThreadChange]
  ) -> RuntimeBridge.RuntimeThreadChangePreview {
    RuntimeBridge.RuntimeThreadChangePreview(
      threadID: "thread-1",
      changes: changes
    )
  }

  private func change(
    path: String,
    canRevert: Bool = true,
    conflictReason: String? = nil
  ) -> RuntimeBridge.RuntimeThreadChange {
    RuntimeBridge.RuntimeThreadChange(
      id: path,
      relativePath: path,
      action: "write_file",
      bytesWritten: 120,
      willDeleteFile: false,
      canRevert: canRevert,
      conflictReason: conflictReason
    )
  }
}
