@testable import PithApp
import XCTest

final class TimelineEvidencePresentationTests: XCTestCase {
  func testWebSearchBadgeLabelsSearchResultAttribution() {
    let badges = TimelineEvidenceBadgePresenter.badges(attributes: [
      "webSearchSourceMode": "searchResultAttribution",
      "pageFetchPerformed": "false",
      "sourceSnapshotAvailable": "false",
    ])

    XCTAssertEqual(badges.count, 1)
    XCTAssertEqual(badges.first?.label, "Search Result Sources")
    XCTAssertEqual(badges.first?.tone, .active)
  }

  func testWebSearchBadgeLabelsSearchResultSnapshotWithoutOverstatingFetchDepth() {
    let badges = TimelineEvidenceBadgePresenter.badges(attributes: [
      "webSearchSourceMode": "searchResultAttribution",
      "pageFetchPerformed": "false",
      "sourceSnapshotAvailable": "true",
      "sourceSnapshotKind": "searchResults",
    ])

    XCTAssertEqual(badges.count, 1)
    XCTAssertEqual(badges.first?.label, "Search Snapshot")
    XCTAssertEqual(badges.first?.tone, .active)
  }

  func testRemoteWriteBadgeUsesPithDerivedStatus() {
    let badges = TimelineEvidenceBadgePresenter.badges(attributes: [
      "remoteWriteStatus": "notSent",
    ])

    XCTAssertEqual(badges.count, 1)
    XCTAssertEqual(badges.first?.label, "Remote Write Not Sent")
    XCTAssertEqual(badges.first?.tone, .warning)
  }

  func testInspectorSummarizesWebSearchSourceDepth() {
    let summary = TimelineInspectorPresenter.selectedEntrySourceSummary(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .assistantMessage,
        title: "Assistant",
        body: "Search result answer.",
        attributes: [
          "sourceAttribution": "web_search",
          "webSearchSourceMode": "searchResultAttribution",
          "pageFetchPerformed": "false",
          "sourceSnapshotAvailable": "true",
          "sourceSnapshotKind": "searchResults",
          "sourceSnapshotHash": "abc123",
          "sourceTitles": "Pith",
          "sourceUrls": "https://example.com/pith",
        ]
      ))
    )

    XCTAssertEqual(
      summary,
      """
      Source mode: searchResultAttribution
      Page fetch: no
      Source snapshot: yes
      Titles: Pith
      URLs: https://example.com/pith
      Snapshot kind: searchResults
      Snapshot hash: abc123
      """
    )
  }

  func testInspectorSummarizesRemoteWriteContract() {
    let summary = TimelineInspectorPresenter.selectedEntryPluginSummary(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .assistantMessage,
        title: "Assistant",
        body: "No remote write was sent.",
        attributes: [
          "commandId": "notion.inspect-page-write",
          "executionKind": "mcp.notion.inspectPageWrite",
          "remoteWrite": "false",
          "remoteWriteStage": "inspectBeforeWrite",
          "remoteWriteStatus": "notSent",
          "remoteWriteRequiresApproval": "true",
          "targetService": "notion",
          "targetTool": "notion.inspectPageWrite",
          "sourceArtifact": "docs/handoff.md",
        ]
      ))
    )

    XCTAssertTrue(summary?.contains("Remote write: notSent") == true)
    XCTAssertTrue(summary?.contains("Remote approval required: true") == true)
    XCTAssertTrue(summary?.contains("Remote write source: docs/handoff.md") == true)
  }
}
