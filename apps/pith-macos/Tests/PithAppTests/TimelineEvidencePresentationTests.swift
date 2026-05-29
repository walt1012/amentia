@testable import PithApp
import XCTest

final class TimelineEvidencePresentationTests: XCTestCase {
  func testActionReceiptBadgeShowsAskMode() {
    let badges = TimelineEvidenceBadgePresenter.badges(attributes: [
      "tool": "read_file",
    ])

    XCTAssertEqual(badges.first?.label, "Ask Mode")
    XCTAssertEqual(badges.first?.tone, .ready)
  }

  func testActionReceiptBadgeShowsApprovalRequired() {
    let badges = TimelineEvidenceBadgePresenter.badges(attributes: [
      "tool": "run_shell",
    ])

    XCTAssertEqual(badges.first?.label, "Approval Required")
    XCTAssertEqual(badges.first?.tone, .warning)
  }

  func testActionReceiptBadgeShowsAutoApprovedMode() {
    let badges = TimelineEvidenceBadgePresenter.badges(attributes: [
      "tool": "write_file",
      "actionApprovalPolicy": "autoApproved",
    ])

    XCTAssertEqual(badges.first?.label, "Auto Approved")
    XCTAssertEqual(badges.first?.tone, .active)
  }

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

  func testInspectorSummarizesActionReceipt() {
    let summary = TimelineInspectorPresenter.selectedEntryActionReceiptSummary(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .tool,
        title: "read_file",
        body: "README.md",
        attributes: [
          "toolName": "read_file",
          "toolKind": "file",
          "workspaceDisplayName": "Pith",
        ]
      ))
    )

    XCTAssertEqual(
      summary,
      """
      Mode: ask-before-change
      Boundary: workspace
      Approval: read-only allowed
      Pith account required: no
      Tool: read_file
      Workspace: Pith
      """
    )
  }

  func testInspectorSummarizesNetworkActionReceiptReason() {
    let summary = TimelineInspectorPresenter.selectedEntryActionReceiptSummary(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .tool,
        title: "web_search",
        body: "latest model",
        attributes: [
          "toolName": "web_search",
          "toolKind": "web",
          "routingReason": "freshPublicInformation",
        ]
      ))
    )

    XCTAssertTrue(summary?.contains("Boundary: network") == true)
    XCTAssertTrue(summary?.contains("Approval: requires enabled plugin permission") == true)
    XCTAssertTrue(summary?.contains("Reason: freshPublicInformation") == true)
  }

  func testInspectorSummarizesAutoApprovedActionReceipt() {
    let summary = TimelineInspectorPresenter.selectedEntryActionReceiptSummary(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .tool,
        title: "write_file result",
        body: "Wrote file.",
        attributes: [
          "tool": "write_file",
          "toolKind": "file",
          "localExecutionSafetyMode": "approvedWorkspaceExecution",
          "actionApprovalPolicy": "autoApproved",
        ]
      ))
    )

    XCTAssertTrue(summary?.contains("Mode: approved workspace execution") == true)
    XCTAssertTrue(summary?.contains("Approval: auto approved") == true)
  }

  func testTimelineCardSummaryKeepsActionEvidenceCompact() {
    let summary = TimelineContextReceiptPresenter.cardSummary(TimelineEntry(
      id: "entry-1",
      kind: .tool,
      title: "write_file result",
      body: "Wrote file.",
      attributes: [
        "tool": "write_file",
        "toolKind": "file",
        "localExecutionSafetyMode": "approvedWorkspaceExecution",
        "actionApprovalPolicy": "autoApproved",
        "memoryContextMode": "ranked",
        "memoryNoteCount": "1",
        "memoryContextCandidateNoteCount": "3",
        "promptTruncated": "true",
      ]
    ))

    XCTAssertEqual(
      summary,
      "write_file | Approved | auto approved | Memory 1/3 | Context compacted"
    )
  }

  func testTimelineCardSummaryNamesBlockedReadOnlyMode() {
    let summary = TimelineContextReceiptPresenter.cardSummary(TimelineEntry(
      id: "entry-1",
      kind: .warning,
      title: "Local Execution Blocked",
      body: "Pith did not run a shell command.",
      attributes: [
        "tool": "run_shell",
        "localExecutionSafetyMode": "explore",
        "actionApprovalPolicy": "blocked",
        "blockReason": "readOnlyMode",
      ]
    ))

    XCTAssertEqual(summary, "Action blocked by read-only mode")
  }

  func testInspectorGroupsContextReceiptSections() {
    let sections = TimelineInspectorPresenter.selectedEntryContextReceiptSections(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .assistantMessage,
        title: "Assistant",
        body: "Search result answer.",
        attributes: [
          "workspaceDisplayName": "Pith",
          "toolName": "web_search",
          "toolKind": "web",
          "sourceAttribution": "web_search",
          "webSearchSourceMode": "searchResultAttribution",
          "routingReason": "freshPublicInformation",
          "pageFetchPerformed": "false",
          "sourceSnapshotAvailable": "true",
          "memoryContextMode": "ranked",
          "memoryNoteCount": "1",
          "memoryNoteTitles": "Project rule",
          "memoryNoteIds": "note-1",
          "memoryRankingScores": "9, 4, 1",
          "observationSourceChars": "4000",
          "observationBudgetChars": "1800",
          "observationTruncated": "true",
          "memoryContextCandidateNoteCount": "3",
          "memoryContextOmittedNoteCount": "2",
          "memoryContextTruncatedNoteCount": "0",
        ]
      ))
    )

    XCTAssertEqual(sections.map(\.title), [
      "Workspace Context",
      "Web Search Sources",
      "Local Action",
      "Memory Context",
      "Context Compaction",
    ])
    XCTAssertTrue(sections[0].body.contains("Workspace: Pith"))
    XCTAssertTrue(sections[1].body.contains("Source snapshot: yes"))
    XCTAssertTrue(sections[1].body.contains("Reason: freshPublicInformation"))
    XCTAssertTrue(sections[2].body.contains("Approval: requires enabled plugin permission"))
    XCTAssertTrue(sections[3].body.contains("Titles: Project rule"))
    XCTAssertTrue(sections[3].body.contains("Ranking scores: 9, 4, 1"))
    XCTAssertTrue(sections[4].body.contains("Observation: 4000/1800 chars | truncated yes"))
    XCTAssertTrue(
      sections[4].body.contains("Memory decision: selected 1/3 notes | omitted 2 | truncated 0")
    )
  }

  func testInspectorSummarizesWorkspaceContextReceipt() {
    let sections = TimelineInspectorPresenter.selectedEntryContextReceiptSections(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .tool,
        title: "search_files result",
        body: "README.md:2 Search target",
        attributes: [
          "tool": "search_files",
          "agentToolName": "search_files",
          "workspaceDisplayName": "Pith",
          "query": "Search target",
          "resultCount": "2",
          "uniquePathCount": "2",
          "maxResults": "12",
          "nextAction": "read_file",
          "nextRelativePath": "README.md",
          "agentStepIndex": "1",
        ]
      ))
    )

    let workspace = sections.first(where: { $0.id == "workspace" })
    XCTAssertEqual(workspace?.title, "Workspace Context")
    XCTAssertTrue(workspace?.body.contains("Tool: search_files") == true)
    XCTAssertTrue(workspace?.body.contains("Query: Search target") == true)
    XCTAssertTrue(workspace?.body.contains("Result count: 2") == true)
    XCTAssertTrue(workspace?.body.contains("Unique paths: 2") == true)
    XCTAssertTrue(workspace?.body.contains("Next action: read_file") == true)
    XCTAssertTrue(workspace?.body.contains("Next path: README.md") == true)
  }

  func testInspectorSummarizesWorkspaceReadReceipt() {
    let sections = TimelineInspectorPresenter.selectedEntryContextReceiptSections(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .tool,
        title: "read_file result",
        body: "# Pith",
        attributes: [
          "tool": "read_file",
          "workspaceDisplayName": "Pith",
          "relativePath": "README.md",
          "maxBytes": "4096",
          "isTruncated": "false",
        ]
      ))
    )

    let workspace = sections.first(where: { $0.id == "workspace" })
    XCTAssertTrue(workspace?.body.contains("Path: README.md") == true)
    XCTAssertTrue(workspace?.body.contains("Max bytes: 4096") == true)
    XCTAssertTrue(workspace?.body.contains("Truncated: no") == true)
  }

  func testInspectorSeparatesCompactionFromMemoryContext() {
    let snapshot = TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
      id: "entry-1",
      kind: .tool,
      title: "read_file result",
      body: "Large file preview.",
      attributes: [
        "memoryContextMode": "compacted",
        "memoryNoteCount": "0",
        "memoryContextCandidateNoteCount": "0",
        "memoryContextSourceNoteCount": "2",
        "memoryContextEstimatedChars": "0",
        "memoryContextBudgetChars": "1228",
        "memoryContextWindowTokens": "4096",
        "promptSourceChars": "9000",
        "promptBudgetChars": "7200",
        "promptTruncated": "true",
        "priorObservationCount": "2",
        "priorObservationSourceChars": "2200",
        "priorObservationBudgetChars": "1800",
        "priorObservationTruncated": "true",
        "priorObservationPaths": "Sources/App.swift\nREADME.md",
      ]
    ))

    let memorySummary = TimelineInspectorPresenter.selectedEntryMemorySummary(snapshot)
    let sections = TimelineInspectorPresenter.selectedEntryContextReceiptSections(snapshot)
    let compaction = sections.first(where: { $0.id == "compaction" })

    XCTAssertTrue(memorySummary?.contains("Memory context: compacted") == true)
    XCTAssertFalse(memorySummary?.contains("Prompt:") == true)
    XCTAssertEqual(compaction?.title, "Context Compaction")
    XCTAssertTrue(compaction?.body.contains("Prompt: 9000/7200 chars | truncated yes") == true)
    XCTAssertTrue(
      compaction?.body.contains("Prior observations: 2200/1800 chars | truncated yes") == true
    )
    XCTAssertTrue(compaction?.body.contains("Prior observation count: 2") == true)
    XCTAssertTrue(compaction?.body.contains("Sources/App.swift") == true)
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
          "routingReason": "freshPublicInformation",
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
      Attribution: web_search
      Reason: freshPublicInformation
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

  func testExternalActionOpensFirstSafeWebSearchSource() {
    let action = TimelineExternalActionPresenter.primaryAction(attributes: [
      "sourceAttribution": "web_search",
      "webSearchSourceMode": "searchResultAttribution",
      "sourceUrls": "file:///tmp/leak https://example.com/pith https://example.com/second",
    ])

    XCTAssertEqual(action?.title, "Open Web Source")
    XCTAssertEqual(action?.copyTitle, "Copy Source Link")
    XCTAssertEqual(action?.url.absoluteString, "https://example.com/pith")
  }

  func testExternalActionRevealsWorkspaceReceiptPath() {
    let action = TimelineExternalActionPresenter.primaryAction(
      attributes: [
        "tool": "read_file",
        "relativePath": "docs/guide.md",
      ],
      workspaceRoot: "/tmp/Pith Workspace"
    )

    XCTAssertEqual(action?.title, "Reveal Source File")
    XCTAssertEqual(action?.copyTitle, "Copy File Path")
    XCTAssertEqual(action?.url.path, "/tmp/Pith Workspace/docs/guide.md")
    XCTAssertEqual(action?.copyValue, "/tmp/Pith Workspace/docs/guide.md")
  }

  func testExternalActionRejectsWorkspacePathEscapes() {
    XCTAssertNil(TimelineExternalActionPresenter.primaryAction(
      attributes: [
        "tool": "read_file",
        "relativePath": "../secrets.txt",
      ],
      workspaceRoot: "/tmp/workspace"
    ))
    XCTAssertNil(TimelineExternalActionPresenter.primaryAction(
      attributes: [
        "tool": "read_file",
        "relativePath": "/etc/passwd",
      ],
      workspaceRoot: "/tmp/workspace"
    ))
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

  func testGenericConnectorProofDoesNotNeedNotionAttributes() {
    let attributes = [
      "remoteWrite": "true",
      "remoteWriteStage": "completed",
      "remoteWriteStatus": "completed",
      "remoteProofKind": "messageApiResponse",
      "remoteProofStatus": "success",
      "remoteProofId": "message-123",
      "remoteProofUrl": "https://chat.example.com/messages/message-123",
      "remoteProofTitle": "Message sent",
      "remoteProofActionTitle": "Open Message",
      "targetService": "team-chat",
      "targetTool": "team-chat.sendMessage",
    ]

    let summary = TimelineInspectorPresenter.selectedEntryPluginSummary(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .assistantMessage,
        title: "Assistant",
        body: "Sent team chat message.",
        attributes: attributes
      ))
    )
    let action = TimelineExternalActionPresenter.primaryAction(attributes: attributes)
    let proof = TimelineExternalActionPresenter.proofSummary(attributes: attributes)

    XCTAssertTrue(summary?.contains("Remote proof ID: message-123") == true)
    XCTAssertTrue(
      summary?.contains("Remote proof URL: https://chat.example.com/messages/message-123") == true
    )
    XCTAssertFalse(summary?.contains("Notion page") == true)
    XCTAssertEqual(action?.title, "Open Message")
    XCTAssertEqual(proof?.title, "Message sent")
    XCTAssertTrue(proof?.detail.contains("ID: message-123") == true)
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
