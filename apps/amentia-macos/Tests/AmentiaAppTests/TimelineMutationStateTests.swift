@testable import AmentiaApp
import XCTest

final class TimelineMutationStateTests: XCTestCase {
  func testDeletingLastSessionReturnsToWelcomeTimeline() {
    var state = TimelineRuntimeState(welcomeState: TimelineSessionState.welcomeState())
    let thread = ThreadSummary(
      id: "thread-1",
      title: "Design Review",
      preview: "Ready",
      workspaceRootPath: "/tmp/project",
      workspaceDisplayName: "Project"
    )
    state.applyWorkspaceThreads([thread])
    state.applyThreadEntries(threadID: thread.id, entries: [
      TimelineEntryFactory.system(
        title: "Session Ready",
        body: "Design Review is ready."
      )
    ])

    state.deleteThread(threadID: thread.id, remainingThreads: [])
    state.appendEntry(
      to: state.selectedThreadID,
      TimelineEntryFactory.system(
        title: "Session Deleted",
        body: "Design Review was removed from Amentia."
      )
    )

    XCTAssertEqual(state.selectedThreadID, TimelineSessionState.welcomeThreadID)
    XCTAssertEqual(state.threads.map(\.id), [TimelineSessionState.welcomeThreadID])
    XCTAssertEqual(state.timeline.first?.title, "Session Deleted")
    XCTAssertEqual(
      state.threadTimelines[TimelineSessionState.welcomeThreadID]?.first?.title,
      "Session Deleted"
    )
    XCTAssertTrue(state.timeline.contains { $0.title == "Start Local Setup" })
    XCTAssertNil(state.activeTurnID)
    XCTAssertNil(state.activeTurnThreadID)
  }
}
