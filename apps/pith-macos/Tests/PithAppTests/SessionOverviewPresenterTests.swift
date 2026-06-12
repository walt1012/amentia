@testable import PithApp
import XCTest

final class SessionOverviewPresenterTests: XCTestCase {
  func testRuntimeThreadPreviewTurnsReadyStatusIntoUserCopy() {
    let preview = SessionOverviewPresenter.runtimeThreadPreview(
      status: "ready",
      workspaceDisplayName: "Pith"
    )

    XCTAssertEqual(preview, "Ready to continue in Pith.")
    XCTAssertFalse(preview.contains("ready"))
  }

  func testRuntimeThreadPreviewPrioritizesActiveWork() {
    let preview = SessionOverviewPresenter.runtimeThreadPreview(
      status: "ready",
      workspaceDisplayName: "Pith",
      hasActiveTurn: true
    )

    XCTAssertEqual(preview, "Working in Pith.")
  }

  func testRuntimeThreadPreviewPrioritizesPendingApproval() {
    XCTAssertEqual(
      SessionOverviewPresenter.runtimeThreadPreview(
        status: "ready",
        pendingApprovalCount: 1
      ),
      "Waiting for your approval."
    )

    XCTAssertEqual(
      SessionOverviewPresenter.runtimeThreadPreview(
        status: "ready",
        pendingApprovalCount: 2
      ),
      "Waiting for 2 approvals."
    )
  }

  func testRuntimeThreadPreviewKeepsUnknownStatusOutOfSidebar() {
    let preview = SessionOverviewPresenter.runtimeThreadPreview(status: "needs_internal_sync")

    XCTAssertEqual(preview, "Ready to continue.")
    XCTAssertFalse(preview.contains("needs_internal_sync"))
  }

  func testRuntimeSummaryMapperUsesHumanThreadPreview() {
    let summary = RuntimeSummaryMapper.threadSummary(from: RuntimeBridge.RuntimeThreadSummary(
      id: "thread-1",
      title: "Session 1",
      status: "ready",
      workspaceRootPath: "/Users/example/Pith",
      workspaceDisplayName: "Pith"
    ))

    XCTAssertEqual(summary.preview, "Ready to continue in Pith.")
  }
}
