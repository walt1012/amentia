@testable import AmentiaApp
import XCTest

final class TimelineEntryFactoryTests: XCTestCase {
  func testRuntimeEntriesUseAgentStepIdentityWhenAvailable() {
    let entries = TimelineEntryFactory.runtimeEntries(
      from: [
        RuntimeBridge.RuntimeTimelineItemResult(
          kind: "plan",
          title: "Plan",
          content: "Inspect README.",
          attributes: [
            "agentStepId": "thread-1-turn-1-step-1",
            "agentToolName": "read_file",
          ]
        ),
      ]
    )

    XCTAssertEqual(
      entries.first?.id,
      "agent-step:thread-1-turn-1-step-1:plan:Plan"
    )
  }

  func testRuntimeEntriesKeepApprovalIdentityFirst() {
    let entries = TimelineEntryFactory.runtimeEntries(
      from: [
        RuntimeBridge.RuntimeTimelineItemResult(
          kind: "approvalRequested",
          title: "Approval Requested",
          content: "Review a write.",
          attributes: [
            "approvalId": "approval-1",
            "agentStepId": "thread-1-turn-1-step-1",
          ]
        ),
      ]
    )

    XCTAssertEqual(
      entries.first?.id,
      "approval:approval-1:approvalRequested:Approval Requested"
    )
  }
}
