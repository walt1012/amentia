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

  func testRemoteWriteBadgeLabelsUnconfirmedWritesHonestly() {
    let badges = TimelineEvidenceBadgePresenter.badges(attributes: [
      "remoteWriteStatus": "unconfirmed",
    ])

    XCTAssertEqual(badges.count, 1)
    XCTAssertEqual(badges.first?.label, "Remote Write Unconfirmed")
    XCTAssertEqual(badges.first?.tone, .warning)
  }

  func testConnectorWorkflowBadgeSupersedesRemoteWriteDraftNoise() {
    let badges = TimelineEvidenceBadgePresenter.badges(attributes: [
      "connectorWorkflowStatus": "prepared",
      "remoteWriteStatus": "notSent",
    ])

    XCTAssertEqual(badges.count, 1)
    XCTAssertEqual(badges.first?.label, "Connector Prepared")
    XCTAssertEqual(badges.first?.tone, .active)
  }

  func testConnectorWorkflowBadgeFlagsRetryNeeded() {
    let badges = TimelineEvidenceBadgePresenter.badges(attributes: [
      "connectorWorkflowStatus": "retryNeeded",
      "remoteWriteStatus": "unconfirmed",
    ])

    XCTAssertEqual(badges.count, 1)
    XCTAssertEqual(badges.first?.label, "Connector Retry Needed")
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
          "remoteProofKind": "notionApiResponse",
          "remoteProofStatus": "notRequested",
          "retryCommandId": "notion-connector::notion.publish-page-draft",
          "retryInput": "{\"parentPageId\":\"page\"}",
          "retryInputEditable": "true",
          "retryInputHint": "Add parentPageId before retrying.",
          "nextCommandId": "notion-connector::notion.publish-page-draft",
          "nextCommandLabel": "Publish to Notion",
          "nextCommandInputHint": "Fill parentPageId before publishing.",
          "nextCommandInputTemplate": "{\"parentPageId\":\"\",\"title\":\"Draft\"}",
          "connectorWorkflowId": "notion.create-page",
          "connectorWorkflowName": "Notion Create Page",
          "connectorWorkflowService": "notion",
          "connectorWorkflowAction": "createPage",
          "connectorWorkflowStage": "inspectBeforeWrite",
          "connectorWorkflowStatus": "inspected",
          "connectorWorkflowTarget": "docs/handoff.md",
          "connectorWorkflowProof": "inspection",
          "targetService": "notion",
          "targetTool": "notion.inspectPageWrite",
          "sourceArtifact": "docs/handoff.md",
        ]
      ))
    )

    XCTAssertTrue(summary?.contains("Remote write: notSent") == true)
    XCTAssertTrue(summary?.contains("Remote approval required: true") == true)
    XCTAssertTrue(summary?.contains("Remote write source: docs/handoff.md") == true)
    XCTAssertTrue(summary?.contains("Remote proof: notRequested | notionApiResponse") == true)
    XCTAssertTrue(
      summary?.contains(
        "Notion Create Page: inspected | stage inspectBeforeWrite | notion createPage"
      ) == true
    )
    XCTAssertTrue(summary?.contains("Workflow target: docs/handoff.md") == true)
    XCTAssertTrue(summary?.contains("Workflow proof: inspection") == true)
    XCTAssertTrue(
      summary?.contains(
        "Next command: Publish to Notion | notion-connector::notion.publish-page-draft"
      ) == true
    )
    XCTAssertTrue(
      summary?.contains("Next input hint: Fill parentPageId before publishing.") == true
    )
    XCTAssertTrue(
      summary?.contains(
        "Next input template: {\"parentPageId\":\"\",\"title\":\"Draft\"}"
      ) == true
    )
    XCTAssertTrue(
      summary?.contains("Retry command: notion-connector::notion.publish-page-draft") == true
    )
    XCTAssertTrue(summary?.contains("Retry input editable: true") == true)
    XCTAssertTrue(summary?.contains("Retry input hint: Add parentPageId before retrying.") == true)
    XCTAssertTrue(summary?.contains("Retry input: {\"parentPageId\":\"page\"}") == true)
  }

  func testInspectorSummarizesCompletedNotionProof() {
    let summary = TimelineInspectorPresenter.selectedEntryPluginSummary(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .assistantMessage,
        title: "Assistant",
        body: "Created Notion page.",
        attributes: [
          "remoteWrite": "true",
          "remoteWriteStage": "completed",
          "remoteWriteStatus": "completed",
          "remoteWriteRequiresApproval": "true",
          "remoteProofKind": "notionApiResponse",
          "remoteProofStatus": "success",
          "notionPageId": "page-123",
          "notionPageUrl": "https://www.notion.so/page-123",
          "notionParentPageId": "parent-456",
          "titleTruncated": "true",
          "bodyTruncated": "false",
          "notionBlockCount": "4",
          "targetService": "notion",
          "targetTool": "notion.createPage",
        ]
      ))
    )

    XCTAssertTrue(summary?.contains("Remote proof: success | notionApiResponse") == true)
    XCTAssertTrue(
      summary?.contains("Notion page: page-123 | https://www.notion.so/page-123") == true
    )
    XCTAssertTrue(summary?.contains("Notion parent: parent-456") == true)
    XCTAssertTrue(summary?.contains("Title truncated: true") == true)
    XCTAssertTrue(summary?.contains("Body truncated: false") == true)
    XCTAssertTrue(summary?.contains("Notion blocks: 4") == true)
  }

  func testExternalActionOpensSuccessfulNotionProofOnly() {
    let action = TimelineExternalActionPresenter.primaryAction(attributes: [
      "remoteProofKind": "notionApiResponse",
      "remoteProofStatus": "success",
      "notionPageId": "page-123",
      "notionPageUrl": "https://www.notion.so/page-123",
    ])

    XCTAssertEqual(action?.title, "Open Notion Page")
    XCTAssertEqual(action?.copyTitle, "Copy Link")
    XCTAssertEqual(action?.url.absoluteString, "https://www.notion.so/page-123")
  }

  func testProofSummaryShowsSuccessfulNotionResult() {
    let summary = TimelineExternalActionPresenter.proofSummary(attributes: [
      "remoteProofKind": "notionApiResponse",
      "remoteProofStatus": "success",
      "notionPageId": "page-123",
      "notionParentPageId": "parent-456",
      "bodyTruncated": "false",
      "notionBlockCount": "4",
    ])

    XCTAssertEqual(summary?.title, "Notion page created")
    XCTAssertEqual(summary?.detail, "Page: page-123 | Parent: parent-456 | Body complete | Blocks: 4")
  }

  func testExternalActionRejectsUntrustedOrIncompleteProof() {
    XCTAssertNil(TimelineExternalActionPresenter.primaryAction(attributes: [
      "remoteProofKind": "notionApiResponse",
      "remoteProofStatus": "success",
      "notionPageId": "page-123",
      "notionPageUrl": "file:///tmp/page",
    ]))
    XCTAssertNil(TimelineExternalActionPresenter.primaryAction(attributes: [
      "remoteProofKind": "notionApiResponse",
      "remoteProofStatus": "success",
      "notionPageId": "page-123",
      "notionPageUrl": "http://www.notion.so/page-123",
    ]))
    XCTAssertNil(TimelineExternalActionPresenter.primaryAction(attributes: [
      "remoteProofKind": "notionApiResponse",
      "remoteProofStatus": "missing",
      "notionPageId": "page-123",
      "notionPageUrl": "https://www.notion.so/page-123",
    ]))
    XCTAssertNil(TimelineExternalActionPresenter.proofSummary(attributes: [
      "remoteProofKind": "notionApiResponse",
      "remoteProofStatus": "missing",
      "notionPageId": "page-123",
    ]))
  }
}
