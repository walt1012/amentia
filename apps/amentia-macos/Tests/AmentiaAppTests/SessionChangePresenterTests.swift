@testable import AmentiaApp
import XCTest

final class SessionChangePresenterTests: XCTestCase {
  func testDeletePromptSeparatesSessionRemovalFromWorkspaceFiles() {
    let prompt = SessionChangePresenter.deletePrompt()

    XCTAssertEqual(prompt.title, "Delete Session?")
    XCTAssertEqual(prompt.confirmButtonTitle, "Delete Session")
    XCTAssertTrue(prompt.message.contains("chat history, activity cards, and unfinished permission requests"))
    XCTAssertTrue(prompt.message.contains("Project files and repositories will not be deleted"))
    XCTAssertTrue(prompt.message.contains("use Review Session Changes before deleting"))
    XCTAssertFalse(prompt.message.contains("timeline"))
    XCTAssertFalse(prompt.message.contains("approval"))
  }

  func testDeleteSuccessCopyNamesSessionAndPreservesProjectFiles() {
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

  func testRevertPromptAllowsCleanRecordedChanges() {
    let prompt = SessionChangePresenter.revertPrompt(for: preview(changes: [
      change(path: "Sources/App.swift"),
      change(path: "README.md"),
    ]))

    XCTAssertEqual(prompt.title, "Revert Session Changes?")
    XCTAssertEqual(prompt.confirmButtonTitle, "Revert Changes")
    XCTAssertTrue(prompt.allowsRevert)
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
