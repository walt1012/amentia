@testable import AmentiaApp
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

  func testRemoteWriteBadgeUsesAmentiaDerivedStatus() {
    let badges = TimelineEvidenceBadgePresenter.badges(attributes: [
      "remoteWriteStatus": "notSent",
    ])

    XCTAssertEqual(badges.count, 1)
    XCTAssertEqual(badges.first?.label, "External Action Not Sent")
    XCTAssertEqual(badges.first?.tone, .warning)
  }

  func testRemoteWriteBadgeLabelsUnconfirmedWritesHonestly() {
    let badges = TimelineEvidenceBadgePresenter.badges(attributes: [
      "remoteWriteStatus": "unconfirmed",
    ])

    XCTAssertEqual(badges.count, 1)
    XCTAssertEqual(badges.first?.label, "External Action Unconfirmed")
    XCTAssertEqual(badges.first?.tone, .warning)
  }

  func testConnectorWorkflowBadgeSupersedesRemoteWriteDraftNoise() {
    let badges = TimelineEvidenceBadgePresenter.badges(attributes: [
      "connectorWorkflowStatus": "prepared",
      "remoteWriteStatus": "notSent",
    ])

    XCTAssertEqual(badges.count, 1)
    XCTAssertEqual(badges.first?.label, "Connection Prepared")
    XCTAssertEqual(badges.first?.tone, .active)
  }

  func testConnectorWorkflowBadgeFlagsRetryNeeded() {
    let badges = TimelineEvidenceBadgePresenter.badges(attributes: [
      "connectorWorkflowStatus": "retryNeeded",
      "remoteWriteStatus": "unconfirmed",
    ])

    XCTAssertEqual(badges.count, 1)
    XCTAssertEqual(badges.first?.label, "Connection Retry Needed")
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
          "workspaceDisplayName": "Amentia",
        ]
      ))
    )

    XCTAssertEqual(
      summary,
      """
      Mode: ask-before-change
      Boundary: project
      Approval: read-only allowed
      Amentia account required: no
      Tool: read_file
      Project: Amentia
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

    XCTAssertTrue(summary?.contains("Mode: approved project execution") == true)
    XCTAssertTrue(summary?.contains("Approval: auto approved") == true)
  }

  func testInspectorSupportDetailsStartWithReadableType() {
    let metadata = TimelineInspectorPresenter.selectedEntryMetadata(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .assistantMessage,
        title: "Assistant",
        body: "Work complete.",
        attributes: [
          "pluginDisplayName": "Notion",
          "remoteWriteStatus": "completed",
        ]
      ))
    )

    XCTAssertTrue(metadata.hasPrefix("Type: assistant message"))
    XCTAssertTrue(metadata.contains("pluginDisplayName: Notion"))
    XCTAssertFalse(metadata.hasPrefix("assistantMessage\n"))
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
        title: "Action Blocked",
      body: "Amentia did not run a shell command.",
      attributes: [
        "tool": "run_shell",
        "localExecutionSafetyMode": "explore",
        "actionApprovalPolicy": "blocked",
        "blockReason": "readOnlyMode",
      ]
    ))

    XCTAssertEqual(summary, "Action blocked by read-only mode")
  }

  func testSandboxProofHidesRawSupportPaths() {
    let summary = TimelineInspectorPresenter.selectedEntrySandboxSummary(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .tool,
        title: "Shell Result",
        body: "Command finished.",
        attributes: [
          "sandboxMode": "workspace-write",
          "sandboxBackend": "macosSeatbelt",
          "sandboxActive": "true",
          "sandboxNetworkPolicy": "network denied",
          "sandboxTempRoot": "/Users/example/work/.amentia/sandbox-tmp",
          "sandboxWritableRoots": "/Users/example/work",
          "sandboxOutputContextMode": "condensed",
          "sandboxOutputSavedBytes": "4096",
          "sandboxOutputArtifactDirectory": "/Users/example/work/.amentia/artifacts",
          "sandboxOutputStdoutArtifactPath": "/Users/example/work/.amentia/artifacts/stdout.txt",
          "sandboxOutputStderrArtifactPath": "/Users/example/work/.amentia/artifacts/stderr.txt",
        ]
      ))
    )

    XCTAssertTrue(summary?.contains("Sandbox: active | network denied") == true)
    XCTAssertTrue(
      summary?.contains("Temporary files stayed inside the selected project.") == true
    )
    XCTAssertTrue(summary?.contains("Full output was saved for troubleshooting.") == true)
    XCTAssertFalse(summary?.contains("/Users/example") == true)
    XCTAssertFalse(summary?.contains("Temp root") == true)
    XCTAssertFalse(summary?.contains("Writable roots") == true)
  }

  func testLocalExecutionRecoveryOffersAskModeForReadOnlyBlock() {
    let action = TimelineLocalExecutionRecoveryPresenter.recoveryAction(
      attributes: [
        "actionApprovalPolicy": "blocked",
        "blockReason": "readOnlyMode",
        "blockedAction": "run a shell command",
        "localExecutionSafetyMode": "explore",
        "retryMessage": "Run the test command.",
      ],
      currentMode: "explore"
    )

    XCTAssertEqual(action?.title, "Switch Mode and Restore Request")
    XCTAssertEqual(action?.targetMode, "askBeforeChange")
    XCTAssertTrue(action?.detail.contains("request approval") == true)
    XCTAssertTrue(action?.detail.contains("run a shell command") == true)
    XCTAssertTrue(action?.detail.contains("original request") == true)
    XCTAssertEqual(action?.retryMessage, "Run the test command.")
  }

  func testLocalExecutionRecoveryWithoutRetryMessageOnlySwitchesMode() {
    let action = TimelineLocalExecutionRecoveryPresenter.recoveryAction(
      attributes: [
        "actionApprovalPolicy": "blocked",
        "blockReason": "readOnlyMode",
        "blockedAction": "prepare a file write",
        "retryMessage": "   ",
      ],
      currentMode: "explore"
    )

    XCTAssertEqual(action?.title, "Switch to Ask Mode")
    XCTAssertEqual(action?.targetMode, "askBeforeChange")
    XCTAssertTrue(action?.detail.contains("request approval") == true)
    XCTAssertFalse(action?.detail.contains("original request") == true)
    XCTAssertNil(action?.retryMessage)
  }

  func testLocalExecutionRecoveryDoesNotBypassPermissionBlocks() {
    XCTAssertNil(TimelineLocalExecutionRecoveryPresenter.recoveryAction(
      attributes: [
        "actionApprovalPolicy": "blocked",
        "blockReason": "missingPermission",
        "blockedAction": "prepare a file write",
      ],
      currentMode: "explore"
    ))
    XCTAssertNil(TimelineLocalExecutionRecoveryPresenter.recoveryAction(
      attributes: [
        "actionApprovalPolicy": "blocked",
        "blockReason": "readOnlyMode",
      ],
      currentMode: "askBeforeChange"
    ))
  }

  func testInspectorUsesReadablePermissionGateLabelAndRecovery() {
    let summary = TimelineInspectorPresenter.selectedEntryPluginSummary(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .warning,
        title: "Plugin Permission Required",
        body: "Amentia could not search the web.",
        attributes: [
          "permissionGate": "requiresPluginPermission",
          "requiredPermission": "tool:web_search",
          "requiredPermissionLabel": "Web Search",
          "permissionRecoveryHint": "Enable the bundled Web Search plugin.",
        ]
      ))
    )

    XCTAssertTrue(summary?.contains("Permission needed: Web Search") == true)
    XCTAssertTrue(summary?.contains("Fix: Enable the bundled Web Search plugin.") == true)
    XCTAssertFalse(summary?.contains("requires tool:web_search") == true)

    let fallbackSummary = TimelineInspectorPresenter.selectedEntryPluginSummary(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-2",
        kind: .warning,
        title: "Permission Required",
        body: "Amentia could not search the web.",
        attributes: [
          "permissionGate": "requiresPluginPermission",
          "requiredPermission": "tool:web_search",
        ]
      ))
    )
    XCTAssertTrue(fallbackSummary?.contains("Permission needed: Web Search") == true)
    XCTAssertFalse(fallbackSummary?.contains("tool:web_search") == true)
  }

  @MainActor
  func testWebSearchPermissionWarningCanEnableBundledPlugin() {
    let viewModel = AppViewModel()
    viewModel.runtimeState = .ready
    viewModel.updatePluginState { state in
      state.plugins = [
        pluginSummary(
          id: "web-search",
          displayName: "Web Search",
          enabled: false,
          capabilities: ["tool:web_search"],
          permissions: ["network.outbound"]
        ),
      ]
    }
    let entry = TimelineEntry(
      id: "entry-1",
      kind: .warning,
      title: "Plugin Permission Required",
      body: "Amentia could not search the web.",
      attributes: [
        "permissionGate": "requiresPluginPermission",
        "pluginId": "web-search",
        "requiredPermission": "tool:web_search",
      ]
    )

    XCTAssertTrue(viewModel.canEnablePlugin(from: entry))
  }

  func testApprovalOutcomeSummarizesApprovedWriteNextStep() {
    let summary = TimelineApprovalOutcomePresenter.summary(attributes: [
      "decision": "approved",
      "action": "write_file",
      "relativePath": "docs/output.txt",
    ])

    XCTAssertEqual(summary?.title, "Approval accepted")
    XCTAssertEqual(summary?.tone, .ready)
    XCTAssertTrue(summary?.detail.contains("`docs/output.txt`") == true)
    XCTAssertTrue(summary?.detail.contains("Review the proof") == true)
  }

  func testApprovalOutcomeSummarizesDeniedActionWithoutRetryConfusion() {
    let summary = TimelineApprovalOutcomePresenter.summary(attributes: [
      "decision": "denied",
      "action": "run_shell",
    ])

    XCTAssertEqual(summary?.title, "Approval denied")
    XCTAssertEqual(summary?.tone, .warning)
    XCTAssertTrue(summary?.detail.contains("No local change was made") == true)
    XCTAssertTrue(summary?.detail.contains("safer version") == true)
  }

  func testInspectorGroupsContextReceiptSections() {
    let sections = TimelineInspectorPresenter.selectedEntryContextReceiptSections(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .assistantMessage,
        title: "Assistant",
        body: "Search result answer.",
        attributes: [
          "workspaceDisplayName": "Amentia",
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
      "Project Context",
      "Web Search Sources",
      "Local Action",
      "Memory Context",
      "Context Compaction",
    ])
    XCTAssertTrue(sections[0].body.contains("Project: Amentia"))
    XCTAssertTrue(sections[1].body.contains("Saved source proof: yes"))
    XCTAssertTrue(
      sections[1].body.contains("Why Amentia searched: fresh public information was useful")
    )
    XCTAssertTrue(sections[2].body.contains("Approval: requires enabled plugin permission"))
    XCTAssertTrue(sections[3].body.contains("Titles: Project rule"))
    XCTAssertTrue(sections[3].body.contains("kept 1 of 3 relevant notes"))
    XCTAssertFalse(sections[3].body.contains("Ranking scores"))
    XCTAssertTrue(
      sections[4].body.contains("Observation context: shortened (4000 -> 1800 characters)")
    )
    XCTAssertTrue(
      sections[4].body.contains("Memory notes: kept 1 of 3, skipped 2, trimmed 0")
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
          "workspaceDisplayName": "Amentia",
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
    XCTAssertEqual(workspace?.title, "Project Context")
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
        body: "# Amentia",
        attributes: [
          "tool": "read_file",
          "workspaceDisplayName": "Amentia",
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

    XCTAssertTrue(memorySummary?.contains("Memory context: compacted notes") == true)
    XCTAssertFalse(memorySummary?.contains("Prompt:") == true)
    XCTAssertEqual(compaction?.title, "Context Compaction")
    XCTAssertTrue(
      compaction?.body.contains("Prompt context: shortened (9000 -> 7200 characters)") == true
    )
    XCTAssertTrue(
      compaction?.body.contains(
        "Prior observations context: shortened (2200 -> 1800 characters)"
      ) == true
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
          "sourceTitles": "Amentia",
          "sourceUrls": "https://example.com/amentia",
        ]
      ))
    )

    XCTAssertEqual(
      summary,
      """
      Search mode: search result sources
      Opened source pages: no
      Saved source proof: yes
      Why Amentia searched: fresh public information was useful
      Sources: Amentia
      Links: https://example.com/amentia
      Proof type: saved search results
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

    XCTAssertTrue(summary?.contains("External action: not sent yet") == true)
    XCTAssertTrue(summary?.contains("Service: Notion via Notion Inspect Page Write") == true)
    XCTAssertTrue(summary?.contains("Approval before external write: yes") == true)
    XCTAssertTrue(summary?.contains("Source file: docs/handoff.md") == true)
    XCTAssertTrue(
      summary?.contains("External proof: not sent yet (Notion confirmation)") == true
    )
    XCTAssertTrue(
      summary?.contains("Notion Create Page: ready for review in review before write.") == true
    )
    XCTAssertTrue(summary?.contains("Target: docs/handoff.md") == true)
    XCTAssertTrue(summary?.contains("Proof: review completed") == true)
    XCTAssertTrue(summary?.contains("Next step: Publish to Notion") == true)
    XCTAssertFalse(summary?.contains("Next step: Publish to Notion (") == true)
    XCTAssertTrue(
      summary?.contains("Input hint: Fill parentPageId before publishing.") == true
    )
    XCTAssertTrue(
      summary?.contains(
        "Draft input: {\"parentPageId\":\"\",\"title\":\"Draft\"}"
      ) == true
    )
    XCTAssertTrue(summary?.contains("Retry step: Notion Publish Page Draft") == true)
    XCTAssertTrue(summary?.contains("Retry input editable: yes") == true)
    XCTAssertTrue(summary?.contains("Retry hint: Add parentPageId before retrying.") == true)
    XCTAssertTrue(summary?.contains("Retry input: {\"parentPageId\":\"page\"}") == true)
    XCTAssertFalse(summary?.contains("inspectBeforeWrite") == true)
    XCTAssertFalse(summary?.contains("notionApiResponse") == true)
    XCTAssertFalse(summary?.contains("notion.inspectPageWrite") == true)
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

    XCTAssertTrue(summary?.contains("External proof: completed (Notion confirmation)") == true)
    XCTAssertTrue(summary?.contains("Service: Notion via Notion Create Page") == true)
    XCTAssertTrue(
      summary?.contains("Notion page: page-123 | https://www.notion.so/page-123") == true
    )
    XCTAssertTrue(summary?.contains("Notion parent: parent-456") == true)
    XCTAssertTrue(summary?.contains("Title was shortened") == true)
    XCTAssertTrue(summary?.contains("Body was complete") == true)
    XCTAssertTrue(summary?.contains("Notion blocks: 4") == true)
    XCTAssertFalse(summary?.contains("notionApiResponse") == true)
    XCTAssertFalse(summary?.contains("notion.createPage") == true)
  }

  func testInspectorUsesReadableConnectorRecoveryCopy() {
    let summary = TimelineInspectorPresenter.selectedEntryPluginSummary(
      TimelineInspectorSnapshot(selectedEntry: TimelineEntry(
        id: "entry-1",
        kind: .assistantMessage,
        title: "Assistant",
        body: "Could not verify the external write.",
        attributes: [
          "remoteWrite": "false",
          "remoteWriteStage": "failedBeforeProof",
          "remoteWriteStatus": "unconfirmed",
          "remoteProofKind": "notionApiResponse",
          "remoteProofStatus": "missing",
          "publishFailureReason": "missingRemoteProof",
          "retryCommandId": "notion-connector::notion.publish-page-draft",
          "retryInputEditable": "false",
          "retryInputHint": "Retry may create a duplicate if the first request completed.",
          "connectorWorkflowName": "Notion Create Page",
          "connectorWorkflowService": "notion",
          "connectorWorkflowStage": "failedBeforeProof",
          "connectorWorkflowStatus": "retryNeeded",
          "connectorWorkflowProof": "missing",
          "connectorWorkflowRecovery": "retry",
          "targetService": "notion",
          "targetTool": "notion.createPage",
        ]
      ))
    )

    XCTAssertTrue(
      summary?.contains(
        "Notion Create Page: needs retry in finished without trusted proof."
      ) == true
    )
    XCTAssertTrue(summary?.contains("Proof: missing") == true)
    XCTAssertTrue(summary?.contains("Recovery: retry available") == true)
    XCTAssertTrue(
      summary?.contains("Publish issue: Amentia could not verify the external result") == true
    )
    XCTAssertFalse(summary?.contains("failedBeforeProof") == true)
    XCTAssertFalse(summary?.contains("retryNeeded") == true)
    XCTAssertFalse(summary?.contains("missingRemoteProof") == true)
    XCTAssertFalse(summary?.contains("notionApiResponse") == true)
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
      "sourceUrls": "file:///tmp/leak https://example.com/amentia https://example.com/second",
    ])

    XCTAssertEqual(action?.title, "Open Web Source")
    XCTAssertEqual(action?.copyTitle, "Copy Source Link")
    XCTAssertEqual(action?.url.absoluteString, "https://example.com/amentia")
  }

  func testExternalActionRevealsWorkspaceReceiptPath() {
    let action = TimelineExternalActionPresenter.primaryAction(
      attributes: [
        "tool": "read_file",
        "relativePath": "docs/guide.md",
      ],
      workspaceRoot: "/tmp/Amentia Workspace"
    )

    XCTAssertEqual(action?.title, "Show Source File")
    XCTAssertEqual(action?.copyTitle, "Copy File Path")
    XCTAssertEqual(action?.url.path, "/tmp/Amentia Workspace/docs/guide.md")
    XCTAssertEqual(action?.copyValue, "/tmp/Amentia Workspace/docs/guide.md")
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

    XCTAssertTrue(summary?.contains("Confirmation: message-123") == true)
    XCTAssertTrue(
      summary?.contains("Proof link: https://chat.example.com/messages/message-123") == true
    )
    XCTAssertFalse(summary?.contains("Notion page") == true)
    XCTAssertEqual(action?.title, "Open Message")
    XCTAssertEqual(proof?.title, "Message sent")
    XCTAssertTrue(proof?.detail.contains("Confirmation: message-123") == true)
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

  private func pluginSummary(
    id: String,
    displayName: String,
    enabled: Bool,
    capabilities: [String],
    permissions: [String]
  ) -> PluginSummary {
    PluginSummary(
      id: id,
      name: id,
      version: "0.1.0",
      displayName: displayName,
      status: "ready",
      description: "Test plugin",
      authorName: "Amentia",
      enabled: enabled,
      defaultEnabled: true,
      capabilities: capabilities,
      permissions: permissions,
      manifestPath: "plugins/bundled/\(id)/amentia-plugin.json",
      provenance: "bundled",
      validationError: nil,
      validationHint: nil
    )
  }
}
