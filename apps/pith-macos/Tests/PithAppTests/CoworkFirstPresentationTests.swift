@testable import PithApp
import XCTest

final class CoworkFirstPresentationTests: XCTestCase {
  func testRuntimeHeaderFramesFirstUseAsCoworkSession() {
    let summary = RuntimeHeaderPresenter.statusSummary(headerSnapshot(hasDraftMessage: false))

    XCTAssertEqual(
      summary,
      "Start with Understand Project, Pick Next Step, or a short cowork prompt."
    )
  }

  func testRuntimeHeaderUsesDailyDriverNextActionAfterSetup() {
    let summary = RuntimeHeaderPresenter.statusSummary(
      headerSnapshot(
        hasDraftMessage: false,
        isWaitingForFirstMessage: false,
        dailyDriverStage: "ready",
        dailyDriverNextAction: "Ask Pith for the next cowork task."
      )
    )

    XCTAssertEqual(summary, "Ask Pith for the next cowork task.")
  }

  func testRuntimeHeaderShowsDetailForToolReadinessIssue() {
    XCTAssertTrue(
      RuntimeHeaderPresenter.shouldShowDetail(
        headerSnapshot(
          hasDraftMessage: false,
          isWaitingForFirstMessage: false,
          runtimeDetail: "Native sandbox status is limited.",
          hasToolReadinessIssue: true
        )
      )
    )
  }

  func testDailyDriverPresenterNamesRetrievalSetup() {
    let summary = DailyDriverStagePresenter.summary(
      stage: "retrieval_setup",
      nextAction: nil
    )

    XCTAssertEqual(
      summary,
      "Enable Web Search so Pith can retrieve current information when needed."
    )
  }

  func testInspectorSessionUsesDailyDriverNextAction() {
    let summary = InspectorSessionPresenter.metaSummary(
      InspectorSessionSnapshot(
        runtimeState: .ready,
        isLocalModelReady: true,
        hasWorkspace: true,
        workspaceDisplayName: "Pith",
        hasRuntimeThreadSelection: true,
        selectedThreadTitle: "Cowork",
        hasActiveTurn: false,
        setupReadyStepCount: SetupFlowState.stepCount,
        setupStepCount: SetupFlowState.stepCount,
        setupProgressDetail: "Ready",
        isWaitingForFirstMessage: false,
        runtimeReadinessStatus: "ready",
        dailyDriverStage: "ready",
        dailyDriverNextAction: "Ask Pith for the next cowork task.",
        runtimeReadinessChecks: [],
        runtimeReadinessMetrics: localExecutionMetrics,
        selectedLocalExecutionSafetyMode: "askBeforeChange"
      )
    )

    XCTAssertTrue(summary.contains("Ask Pith for the next cowork task."))
    XCTAssertTrue(summary.contains("Safety Ask, no account"))
    XCTAssertTrue(summary.contains("Mode Ask"))
  }

  func testToolReadinessShowsLocalExecutionModeWhenReady() {
    let detail = RuntimeToolReadinessPresenter.timelineDetail(
      readyChecks(),
      metrics: localExecutionMetrics
    )
    let summary = RuntimeToolReadinessPresenter.inspectorSummary(
      readyChecks(),
      metrics: localExecutionMetrics
    )

    XCTAssertEqual(detail, "Ask")
    XCTAssertEqual(summary, "Safety Ask, no account")
  }

  func testReadinessStripStaysVisibleForFirstRequest() {
    XCTAssertTrue(
      RuntimeReadinessStripPresenter.shouldShow(
        setupProgressVisible: false,
        isWaitingForFirstMessage: true,
        runtimeReadinessChecks: readyChecks()
      )
    )
  }

  func testRuntimeReadinessStepUsesProductLanguage() {
    let runtimeStep = RuntimeReadinessPresenter.steps(readinessSnapshotForFirstRequest())
      .first { $0.id == "runtime" }

    XCTAssertEqual(runtimeStep?.label, "Pith")
    XCTAssertNotEqual(runtimeStep?.label, "Service")
    XCTAssertNotEqual(runtimeStep?.label, "Engine")
  }

  func testSetupProgressFramesFirstRequestAsStarterChoice() {
    let detail = SetupProgressPresenter.detail(
      setupProgressSnapshot(
        isWaitingForFirstMessage: true,
        hasDraft: false
      )
    )

    XCTAssertEqual(detail, "Next: Pick Start")
  }

  func testSetupProgressSendsExistingFirstRequestDraft() {
    let detail = SetupProgressPresenter.detail(
      setupProgressSnapshot(
        isWaitingForFirstMessage: true,
        hasDraft: true
      )
    )

    XCTAssertEqual(detail, "Next: Send Prompt")
  }

  func testSetupProgressNamesModelDownloadForFirstUse() {
    let detail = SetupProgressPresenter.detail(
      setupProgressSnapshot(
        isLocalModelReady: false,
        modelReadinessDetail: "Download"
      )
    )

    XCTAssertEqual(detail, "Next: Download Model")
  }

  func testSetupProgressNamesModelRepairForFirstUse() {
    let detail = SetupProgressPresenter.detail(
      setupProgressSnapshot(
        isLocalModelReady: false,
        modelReadinessDetail: "Repair"
      )
    )

    XCTAssertEqual(detail, "Next: Repair Model")
  }

  func testSetupProgressUsesWorkLanguageWhileBusy() {
    let activeDetail = SetupProgressPresenter.detail(
      setupProgressSnapshot(hasActiveTurn: true)
    )
    let modelDetail = SetupProgressPresenter.detail(
      setupProgressSnapshot(
        isLocalModelReady: false,
        modelReadinessDetail: "Working"
      )
    )

    XCTAssertEqual(activeDetail, "Working")
    XCTAssertEqual(modelDetail, "Next: Finish Work")
    XCTAssertFalse(activeDetail.contains("Turn"))
    XCTAssertFalse(modelDetail.contains("Turn"))
  }

  func testTimelineResponsePreviewAvoidsTurnIDs() {
    XCTAssertEqual(
      TimelineEventPresenter.turnPreview(turnID: "turn-1", activeTurnID: nil),
      "Response ready"
    )
    XCTAssertEqual(
      TimelineEventPresenter.turnPreview(turnID: "turn-1", activeTurnID: "turn-1"),
      "Response in progress"
    )
  }

  func testTimelineFailureUsesRequestLanguage() {
    let entry = TimelineEventPresenter.turnFailed(
      error: RuntimeBridge.RuntimeError.rpc("failed")
    )

    XCTAssertEqual(entry.title, "Request Failed")
    XCTAssertFalse(entry.title.contains("Turn"))
  }

  func testReadinessStripStaysVisibleForToolSetup() {
    XCTAssertTrue(
      RuntimeReadinessStripPresenter.shouldShow(
        setupProgressVisible: false,
        isWaitingForFirstMessage: false,
        runtimeReadinessChecks: [
          RuntimeReadinessCheckSummary(
            id: "webSearch",
            title: "Web Search",
            status: "setup_required",
            detail: "Enable Web Search"
          )
        ]
      )
    )
  }

  func testReadinessStripHidesAfterDailyDriverReady() {
    XCTAssertFalse(
      RuntimeReadinessStripPresenter.shouldShow(
        setupProgressVisible: false,
        isWaitingForFirstMessage: false,
        runtimeReadinessChecks: readyChecks()
      )
    )
  }

  func testOptionalPluginsDoNotKeepReadinessStripVisible() {
    let optionalPluginChecks = readyChecks() + [
      RuntimeReadinessCheckSummary(
        id: "plugins",
        title: "Plugins",
        status: "optional",
        detail: "No plugin action capabilities are required."
      )
    ]

    XCTAssertFalse(
      RuntimeReadinessStripPresenter.shouldShow(
        setupProgressVisible: false,
        isWaitingForFirstMessage: false,
        runtimeReadinessChecks: optionalPluginChecks
      )
    )
    XCTAssertEqual(
      RuntimeToolReadinessPresenter.inspectorSummary(
        optionalPluginChecks,
        metrics: localExecutionMetrics
      ),
      "Safety Ask, no account"
    )
  }

  func testToolReadinessActionOpensWebSearchAccess() {
    let step = ReadinessStepSummary(
      id: "tools",
      label: "Actions",
      detail: "Web Setup",
      tone: .warning
    )
    let snapshot = readinessActionSnapshot(checks: [
      RuntimeReadinessCheckSummary(
        id: "webSearch",
        title: "Web Search",
        status: "setup_required",
        detail: "Enable Web Search"
      )
    ])
    let action = RuntimeReadinessActionPlanner.action(for: step, snapshot: snapshot)

    XCTAssertEqual(RuntimeReadinessActionPlanner.title(for: action, snapshot: snapshot), "Access")
    XCTAssertTrue(RuntimeReadinessActionPlanner.canRun(action, snapshot: snapshot))
  }

  func testToolReadinessActionEnablesWebSearchWhenAvailable() {
    let step = ReadinessStepSummary(
      id: "tools",
      label: "Actions",
      detail: "Web Setup",
      tone: .warning
    )
    let snapshot = readinessActionSnapshot(
      checks: [
        RuntimeReadinessCheckSummary(
          id: "webSearch",
          title: "Web Search",
          status: "setup_required",
          detail: "Enable Web Search"
        )
      ],
      canEnableWebSearchPlugin: true
    )
    let action = RuntimeReadinessActionPlanner.action(for: step, snapshot: snapshot)

    XCTAssertEqual(RuntimeReadinessActionPlanner.title(for: action, snapshot: snapshot), "Enable")
    XCTAssertTrue(RuntimeReadinessActionPlanner.canRun(action, snapshot: snapshot))
  }

  func testToolReadinessActionOpensPluginCommands() {
    let step = ReadinessStepSummary(
      id: "tools",
      label: "Actions",
      detail: "Plugins Setup",
      tone: .warning
    )
    let snapshot = readinessActionSnapshot(checks: [
      RuntimeReadinessCheckSummary(
        id: "plugins",
        title: "Plugins",
        status: "setup_required",
        detail: "Enable action capability"
      )
    ])
    let action = RuntimeReadinessActionPlanner.action(for: step, snapshot: snapshot)

    XCTAssertEqual(RuntimeReadinessActionPlanner.title(for: action, snapshot: snapshot), "Commands")
    XCTAssertTrue(RuntimeReadinessActionPlanner.canRun(action, snapshot: snapshot))
  }

  func testToolReadinessActionInspectsNativeSandbox() {
    let step = ReadinessStepSummary(
      id: "tools",
      label: "Actions",
      detail: "Sandbox Limited",
      tone: .warning
    )
    let snapshot = readinessActionSnapshot(checks: [
      RuntimeReadinessCheckSummary(
        id: "nativeSandbox",
        title: "Native Sandbox",
        status: "limited",
        detail: "Native sandbox unavailable"
      )
    ])
    let action = RuntimeReadinessActionPlanner.action(for: step, snapshot: snapshot)

    XCTAssertEqual(RuntimeReadinessActionPlanner.title(for: action, snapshot: snapshot), "Inspect")
    XCTAssertTrue(RuntimeReadinessActionPlanner.canRun(action, snapshot: snapshot))
  }

  func testFirstRequestReadinessStepNamesPromptChoice() {
    guard let step = RuntimeReadinessPresenter.steps(readinessSnapshotForFirstRequest())
      .first(where: { $0.id == "first-request" })
    else {
      XCTFail("Expected first-request readiness step")
      return
    }

    XCTAssertEqual(step.detail, "Choose")
    let actionSnapshot = readinessActionSnapshot(
      checks: readyChecks(),
      isWaitingForFirstMessage: true
    )

    let action = RuntimeReadinessActionPlanner.action(
      for: step,
      snapshot: actionSnapshot
    )

    XCTAssertNil(action)
    XCTAssertNil(RuntimeReadinessActionPlanner.title(for: action, snapshot: actionSnapshot))
    XCTAssertFalse(RuntimeReadinessActionPlanner.canRun(action, snapshot: actionSnapshot))
  }

  func testFirstRequestReadinessStepOnlySendsExistingDraft() {
    let step = ReadinessStepSummary(
      id: "first-request",
      label: "First Prompt",
      detail: "Draft",
      tone: .warning
    )
    let actionSnapshot = readinessActionSnapshot(
      checks: readyChecks(),
      isWaitingForFirstMessage: true,
      hasDraftMessage: true
    )

    let action = RuntimeReadinessActionPlanner.action(
      for: step,
      snapshot: actionSnapshot
    )

    XCTAssertEqual(
      RuntimeReadinessActionPlanner.title(for: action, snapshot: actionSnapshot),
      "Send"
    )
    XCTAssertTrue(RuntimeReadinessActionPlanner.canRun(action, snapshot: actionSnapshot))
  }

  func testComposerFramesDraftAsCoworkPrompt() {
    let placeholder = ComposerStatusPresenter.placeholder(composerSnapshot(hasDraftMessage: true))
    let summary = ComposerStatusPresenter.statusSummary(composerSnapshot(hasDraftMessage: true))

    XCTAssertEqual(placeholder, "Review the first cowork prompt, then send")
    XCTAssertEqual(summary, "Review the starter prompt, then start the cowork session.")
  }

  func testComposerFramesRestoredLocalExecutionDraft() {
    let placeholder = ComposerStatusPresenter.placeholder(composerSnapshot(
      hasDraftMessage: true,
      hasRestoredLocalExecutionDraft: true
    ))
    let summary = ComposerStatusPresenter.statusSummary(composerSnapshot(
      hasDraftMessage: true,
      hasRestoredLocalExecutionDraft: true
    ))

    XCTAssertEqual(placeholder, "Review the restored prompt, then send")
    XCTAssertEqual(summary, "Ask mode is ready. Review the restored prompt, then send it.")
  }

  func testActiveWorkCopyAvoidsLocalExecutionTerminology() {
    let composer = composerSnapshot(hasDraftMessage: false, hasActiveTurn: true)
    let inspector = InspectorSessionSnapshot(
      runtimeState: .ready,
      isLocalModelReady: true,
      hasWorkspace: true,
      workspaceDisplayName: "Pith",
      hasRuntimeThreadSelection: true,
      selectedThreadTitle: "Cowork",
      hasActiveTurn: true,
      setupReadyStepCount: SetupFlowState.stepCount,
      setupStepCount: SetupFlowState.stepCount,
      setupProgressDetail: "Working",
      isWaitingForFirstMessage: false,
      runtimeReadinessStatus: "running",
      dailyDriverStage: "running",
      dailyDriverNextAction: nil,
      runtimeReadinessChecks: [],
      runtimeReadinessMetrics: localExecutionMetrics,
      selectedLocalExecutionSafetyMode: "askBeforeChange"
    )

    XCTAssertEqual(
      ComposerStatusPresenter.placeholder(composer),
      "Pith is working. Cancel to stop it."
    )
    XCTAssertEqual(
      ComposerStatusPresenter.statusSummary(composer),
      "Pith is working. Cancel the request if it is no longer useful."
    )
    XCTAssertEqual(InspectorSessionPresenter.title(inspector), "Pith Is Working")
    XCTAssertFalse(ComposerStatusPresenter.placeholder(composer).contains("local execution"))
    XCTAssertFalse(InspectorSessionPresenter.title(inspector).contains("Execution"))
  }

  func testActionSafetyFallbackAvoidsExecutionTerminology() {
    XCTAssertEqual(
      LocalExecutionSafetyModePresenter.userDetail("custom"),
      "Custom action safety mode."
    )
  }

  func testFirstReadyTimelineEventFramesCoworkRequest() {
    let entry = TimelineEventPresenter.firstRequestReady()

    XCTAssertEqual(entry.title, "Cowork Session Ready")
    XCTAssertTrue(entry.body.contains("cowork prompt"))
  }

  func testRuntimeMissingErrorAvoidsDeveloperOverrideInUserCopy() {
    let error = RuntimeBridge.RuntimeError.runtimePathMissing
    let entry = TimelineEventPresenter.runtimeLaunchFailed(error: error)

    XCTAssertTrue(error.localizedDescription.contains("download a fresh installer"))
    XCTAssertFalse(error.localizedDescription.contains("PITH_RUNTIME_PATH"))
    XCTAssertFalse(entry.body.contains("PITH_RUNTIME_PATH"))
  }

  func testPluginSurfaceSummarySeparatesBundleFromCapabilities() {
    let surface = PluginSurfaceClassifier.summary(
      capabilities: [
        "command:run",
        "connector:notion",
        "skill:workspace",
        "mcp_server:notion",
        "tool:web_search",
        "hook:verify",
        "connector_workflow:publish",
      ],
      permissions: ["network.outbound"]
    )

    XCTAssertEqual(
      surface.summary,
      "1 action | 1 connection | 1 skill | 1 MCP server | 1 tool | 1 check | 1 workflow | 1 permission"
    )
    XCTAssertEqual(surface.preferredSection, .commands)
  }

  func testPluginDashboardUsesHumanCapabilityLanguage() {
    let snapshot = pluginDashboardSnapshot()
    let detail = PluginDashboardPresenter.pluginDetailSummary(snapshot)
    let registry = PluginDashboardPresenter.registryDetailSummary(snapshot)
    let permissions = PluginDashboardPresenter.permissionDetailSummary(snapshot)

    XCTAssertEqual(PluginDashboardPresenter.pluginCountSummary(snapshot), "1 plugin ready")
    XCTAssertTrue(detail.contains("1 action"))
    XCTAssertTrue(detail.contains("1 connection"))
    XCTAssertTrue(detail.contains("1 skill"))
    XCTAssertTrue(detail.contains("1 MCP server"))
    XCTAssertTrue(permissions.contains("Network access"))
    XCTAssertFalse(detail.contains("command:"))
    XCTAssertFalse(detail.contains("mcp_server:"))
    XCTAssertFalse(permissions.contains("network.outbound"))
    XCTAssertEqual(registry, "1 action | 1 connection | 1 skill | 1 MCP server")
  }

  func testPluginDashboardHidesRawConnectionIdentifiers() {
    let snapshot = pluginDashboardSnapshot(
      connectors: [
        pluginConnectorSummary(
          id: "notion::main",
          displayName: "Notion",
          status: "needsAuth",
          authStatus: "needsAuth",
          credentialPresent: false
        )
      ],
      commands: [
        pluginCommandSummary(requiredConnectorIds: ["notion::missing"])
      ]
    )

    let connectorDetail = PluginDashboardPresenter.connectorDetailSummary(snapshot)
    let commandDetail = PluginDashboardPresenter.commandDetailSummary(snapshot)

    XCTAssertTrue(connectorDetail.contains("Authorization: needs sign in"))
    XCTAssertFalse(connectorDetail.contains("needsAuth"))
    XCTAssertTrue(commandDetail.contains("A required connection is missing."))
    XCTAssertFalse(commandDetail.contains("notion::missing"))
    XCTAssertFalse(commandDetail.contains("connectors"))
  }

  func testPluginValidationFallbackAvoidsUnknownErrorCopy() {
    let snapshot = pluginDashboardSnapshot(
      plugins: [
        pluginSummary(
          status: "invalid",
          validationError: nil,
          validationHint: nil
        )
      ]
    )

    let detail = PluginDashboardPresenter.invalidPluginDetailSummary(snapshot)

    XCTAssertTrue(detail.contains("Setup needs review."))
    XCTAssertFalse(detail.contains("Unknown validation error"))
  }

  func testPluginCapabilityPresenterHidesRawMetadata() {
    let capability = PluginCapabilitySummary(
      id: "notion::mcp_server::notion-pages",
      kind: "mcp_server",
      identifier: "notion-pages",
      pluginID: "notion",
      pluginDisplayName: "Notion",
      permissions: ["network.outbound"],
      manifestPath: "/tmp/notion/pith-plugin.json",
      metadata: [
        "definitionPath": "/tmp/notion/mcp/notion-pages.json",
        "definitionStatus": "missing",
        "definitionError": "failed to read /tmp/notion/mcp/notion-pages.json",
        "surface": "mcp_server",
      ]
    )

    let title = PluginCapabilityPresenter.title(capability)
    let summary = PluginCapabilityPresenter.diagnosticSummary(capability) ?? ""
    let detail = PluginCapabilityPresenter.diagnosticDetail(capability) ?? ""
    let visibleText = [title, summary, detail].joined(separator: "\n")

    XCTAssertEqual(title, "MCP server")
    XCTAssertTrue(detail.contains("plugin setup"))
    XCTAssertFalse(visibleText.contains("notion-pages"))
    XCTAssertFalse(visibleText.contains("/tmp/notion"))
    XCTAssertFalse(visibleText.contains("definitionPath"))
    XCTAssertFalse(visibleText.contains("mcp_server"))
  }

  func testPluginInstallConfirmationAvoidsRawPathsAndManifestTerms() {
    let preview = PluginInstallPreview(
      pluginID: "notion",
      sourcePath: "/Users/example/Desktop/notion-plugin",
      manifestPath: "/Users/example/Desktop/notion-plugin/pith-plugin.json",
      installPath: "/Users/example/Library/Application Support/Pith/plugins/notion",
      displayName: "Notion",
      version: "1.0.0",
      description: "Work with Notion locally.",
      authorName: "Pith",
      capabilities: [
        "command:notion.run",
        "connector:notion",
        "mcp_server:notion",
      ],
      permissions: [
        "network.outbound",
        "mcp.connect",
      ],
      defaultEnabled: true,
      installStatus: "ready",
      installBlocker: nil,
      installRepairHint: nil
    )

    let text = PluginInstallDialogPresenter.installInformativeText(preview: preview)

    XCTAssertTrue(text.contains("Source: Local folder you selected"))
    XCTAssertTrue(text.contains("Installed: Stored in Pith support data"))
    XCTAssertTrue(text.contains("Capabilities: 1 action | 1 connection | 1 MCP server"))
    XCTAssertTrue(text.contains("Permissions: MCP access, Network access"))
    XCTAssertFalse(text.contains("/Users/example"))
    XCTAssertFalse(text.contains("command:"))
    XCTAssertFalse(text.contains("mcp_server:"))
    XCTAssertFalse(text.contains("network.outbound"))
    XCTAssertFalse(text.contains("1 permission"))
  }

  func testProjectAndMemoryTimelineAvoidRawSupportDetails() {
    let workspace = RuntimeBridge.RuntimeWorkspace(
      rootPath: "/Users/example/Projects/Pith",
      displayName: "Pith",
      threadCount: 1
    )
    let opened = TimelineEventPresenter.workspaceOpened(workspace)
    let restored = RuntimeLaunchAnnotationFactory.entries(RuntimeLaunchAnnotationSnapshot(
      serverName: "pith-runtime",
      serverVersion: "0.1.0",
      shouldAnnotateSetupLaunch: false,
      restoredWorkspace: WorkspaceSummary(
        rootPath: workspace.rootPath,
        displayName: workspace.displayName
      ),
      skippedWorkspaceRestorePath: nil,
      workspaceRestoreErrorDetail: nil,
      modelHealth: nil,
      isLocalModelReady: false,
      localModelRequiredSummary: "Choose a local model to continue."
    )).first { $0.title == "Project Restored" }
    let memory = TimelineEventPresenter.memoryNoteSaved(RuntimeBridge.RuntimeMemoryNote(
      id: "note-1",
      title: "Review habit",
      body: "Use concise project summaries.",
      scope: "Pith",
      source: "workspace",
      createdAt: 1,
      tags: ["workspace"]
    ))

    XCTAssertEqual(opened.body, "Opened Pith as the active project.")
    XCTAssertEqual(opened.attributes["workspacePath"], "/Users/example/Projects/Pith")
    XCTAssertFalse(opened.body.contains("/Users/example"))
    XCTAssertEqual(restored?.body, "Restored Pith as the active project.")
    XCTAssertFalse(restored?.body.contains("/Users/example") == true)
    XCTAssertEqual(memory.body, "Saved project memory note Review habit.")
    XCTAssertFalse(memory.body.contains("built-in workspace"))
  }

  func testTimelineStatusCopyAvoidsInternalExecutionTerms() {
    let cancelled = TimelineEventPresenter.pendingTurnCancelled()

    XCTAssertEqual(TimelineEventPresenter.pendingTurnCancelledDetail, "Request cancelled.")
    XCTAssertEqual(TimelineEventPresenter.cancellingTurnDetail, "Cancelling request...")
    XCTAssertEqual(cancelled.body, "The pending request was cancelled before it finished.")
    XCTAssertEqual(
      TimelineEventPresenter.pluginCommandNeedsExecutionContractDetail,
      "Plugin action needs a supported local runner before it can run."
    )
    XCTAssertFalse(cancelled.body.contains("local execution"))
    XCTAssertFalse(
      TimelineEventPresenter.pluginCommandNeedsExecutionContractDetail.contains("contract")
    )
  }

  func testPluginInstallRepairHintsUseSetupLanguage() {
    XCTAssertEqual(
      PluginInstallDialogPresenter.repairHint(for: TestPresentationError(
        message: "Plugin does not contain pith-plugin.json"
      )),
      "Choose a complete plugin folder, or select the plugin setup file directly."
    )
    XCTAssertEqual(
      PluginInstallDialogPresenter.repairHint(for: TestPresentationError(
        message: "Plugin cannot contain nested pith-plugin.json manifests"
      )),
      "Remove nested plugin bundles before installing. Install each plugin as its own folder."
    )
    XCTAssertEqual(
      PluginInstallDialogPresenter.repairHint(for: TestPresentationError(
        message: "Select a plugin folder or a pith-plugin.json manifest"
      )),
      "Point the installer at a plugin directory or the plugin setup file."
    )
  }

  func testPluginLifecycleTimelineUsesProductLanguage() {
    let preview = PluginInstallPreview(
      pluginID: "notion",
      sourcePath: "/Users/example/Desktop/notion-plugin",
      manifestPath: "/Users/example/Desktop/notion-plugin/pith-plugin.json",
      installPath: "/Users/example/Library/Application Support/Pith/plugins/notion",
      displayName: "Notion",
      version: "1.0.0",
      description: "Work with Notion locally.",
      authorName: "Pith",
      capabilities: [
        "command:notion.run",
        "connector:notion",
      ],
      permissions: [
        "network.outbound",
      ],
      defaultEnabled: true,
      installStatus: "ready",
      installBlocker: nil,
      installRepairHint: nil
    )
    let plugin = RuntimeBridge.RuntimePlugin(
      id: "notion",
      name: "notion",
      version: "1.0.0",
      displayName: "Notion",
      status: "ready",
      description: "Work with Notion locally.",
      authorName: "Pith",
      enabled: true,
      defaultEnabled: true,
      capabilities: preview.capabilities,
      permissions: preview.permissions,
      manifestPath: preview.manifestPath,
      provenance: "local",
      validationError: nil,
      validationHint: nil
    )
    let installed = TimelineEventPresenter.pluginInstalled(plugin, preview: preview)
    let removed = TimelineEventPresenter.pluginRemoved(
      RuntimeBridge.RuntimePluginRemoval(
        pluginID: "notion",
        displayName: "Notion",
        removedPath: "/Users/example/Library/Application Support/Pith/plugins/notion"
      )
    )

    XCTAssertEqual(installed.title, "Plugin Installed")
    XCTAssertTrue(installed.body.contains("Capabilities: 1 action | 1 connection"))
    XCTAssertTrue(installed.body.contains("Permissions: Network access"))
    XCTAssertFalse(installed.body.contains("Surface:"))
    XCTAssertFalse(installed.body.contains("/Users/example"))
    XCTAssertEqual(removed.title, "Plugin Removed")
    XCTAssertFalse(removed.body.contains("Removed Path"))
    XCTAssertFalse(removed.body.contains("/Users/example"))
  }

  func testConnectionEvidenceUsesAuthorizationSummary() {
    let lines = TimelineConnectorEvidencePresenter.summaryLines(attributes: [
      "connectorId": "notion",
      "connectorService": "notion",
      "credentialBinding": "env-bound",
    ])

    XCTAssertEqual(
      lines.first,
      "Connection: Notion. Authorization: available locally."
    )
    XCTAssertFalse(lines.joined(separator: "\n").contains("env-bound"))
    XCTAssertFalse(lines.joined(separator: "\n").contains("credentialBinding"))
  }

  private func headerSnapshot(
    hasDraftMessage: Bool,
    isWaitingForFirstMessage: Bool = true,
    runtimeDetail: String = "",
    hasToolReadinessIssue: Bool = false,
    dailyDriverStage: String? = nil,
    dailyDriverNextAction: String? = nil
  ) -> RuntimeHeaderSnapshot {
    RuntimeHeaderSnapshot(
      runtimeState: .ready,
      runtimeDetail: runtimeDetail,
      modelSetupSummary: "Model ready",
      isLocalModelReady: true,
      hasWorkspace: true,
      hasRuntimeThreadSelection: true,
      hasActiveTurn: false,
      isWaitingForFirstMessage: isWaitingForFirstMessage,
      hasDraftMessage: hasDraftMessage,
      isWorkspaceSearching: false,
      hasModelDownload: false,
      hasPausedModelDownload: false,
      hasToolReadinessIssue: hasToolReadinessIssue,
      dailyDriverStage: dailyDriverStage,
      dailyDriverNextAction: dailyDriverNextAction
    )
  }

  private func composerSnapshot(
    hasDraftMessage: Bool,
    hasRestoredLocalExecutionDraft: Bool = false,
    hasActiveTurn: Bool = false
  ) -> ComposerStatusSnapshot {
    ComposerStatusSnapshot(
      runtimeState: .ready,
      modelSetupTitle: "Download model",
      modelSetupSummary: "Model ready.",
      isLocalModelReady: true,
      hasWorkspace: true,
      hasRuntimeThreadSelection: true,
      hasActiveTurn: hasActiveTurn,
      isWaitingForFirstMessage: true,
      hasDraftMessage: hasDraftMessage,
      hasRestoredLocalExecutionDraft: hasRestoredLocalExecutionDraft
    )
  }

  private func setupProgressSnapshot(
    isWaitingForFirstMessage: Bool = true,
    hasDraft: Bool = false,
    isLocalModelReady: Bool = true,
    hasActiveTurn: Bool = false,
    modelReadinessDetail: String = "Ready"
  ) -> SetupProgressSnapshot {
    SetupProgressSnapshot(
      readyStepCount: SetupFlowState.stepCount - 1,
      stepCount: SetupFlowState.stepCount,
      runtimeState: .ready,
      showsRuntimeActivity: false,
      isLocalModelReady: isLocalModelReady,
      hasWorkspace: true,
      hasRuntimeThreadSelection: true,
      hasActiveTurn: hasActiveTurn,
      isWaitingForFirstMessage: isWaitingForFirstMessage,
      hasDraft: hasDraft,
      modelReadinessDetail: modelReadinessDetail
    )
  }

  private func readyChecks() -> [RuntimeReadinessCheckSummary] {
    [
      RuntimeReadinessCheckSummary(
        id: "executionControls",
        title: "Action Safety",
        status: "ready",
        detail: "Ready"
      )
    ]
  }

  private func readinessActionSnapshot(
    checks: [RuntimeReadinessCheckSummary],
    canEnableWebSearchPlugin: Bool = false,
    isWaitingForFirstMessage: Bool = false,
    hasDraftMessage: Bool = false
  ) -> RuntimeReadinessActionSnapshot {
    RuntimeReadinessActionSnapshot(
      runtimeState: .ready,
      isLocalModelReady: true,
      hasWorkspace: true,
      hasRuntimeThreadSelection: true,
      canLaunchRuntime: false,
      canRunModelSetupAction: false,
      canOpenWorkspace: false,
      canCreateThread: false,
      canUseComposer: true,
      isWaitingForFirstMessage: isWaitingForFirstMessage,
      hasDraftMessage: hasDraftMessage,
      runtimeReadinessChecks: checks,
      canEnableWebSearchPlugin: canEnableWebSearchPlugin,
      runtimeLaunchButtonTitle: "Start Pith",
      modelSetupActionTitle: nil
    )
  }

  private func readinessSnapshotForFirstRequest() -> RuntimeReadinessSnapshot {
    RuntimeReadinessSnapshot(
      runtimeState: .ready,
      modelReadinessDetail: "Ready",
      modelTone: .ready,
      workspaceDisplayName: "Pith",
      isLocalModelReady: true,
      hasWorkspace: true,
      hasRuntimeThreadSelection: true,
      hasActiveTurn: false,
      isWaitingForFirstMessage: true,
      hasDraftMessage: false,
      runtimeReadinessChecks: readyChecks(),
      runtimeReadinessMetrics: localExecutionMetrics
    )
  }

  private let localExecutionMetrics = [
    "pithAccountRequired": "false",
    "defaultLocalExecutionSafetyMode": "askBeforeChange",
    "localExecutionSafetyModes": "explore,askBeforeChange,approvedWorkspaceExecution",
  ]

  private func pluginDashboardSnapshot(
    plugins: [PluginSummary]? = nil,
    connectors: [PluginConnectorSummary] = [],
    commands: [PluginCommandSummary] = []
  ) -> PluginDashboardSnapshot {
    PluginDashboardSnapshot(
      plugins: plugins ?? [pluginSummary()],
      registrySummary: PluginCapabilityRegistrySummary(
        enabledPluginCount: 1,
        totalCapabilityCount: 4,
        capabilityCountsByKind: [
          "command": 1,
          "connector": 1,
          "skill": 1,
          "mcp_server": 1,
        ]
      ),
      capabilities: [],
      connectors: connectors,
      commands: commands,
      hooks: [],
      diagnostics: [],
      refreshRecoveryAttributes: [:],
      hasLifecycleOperation: false
    )
  }

  private func pluginSummary(
    status: String = "ready",
    validationError: String? = nil,
    validationHint: String? = nil
  ) -> PluginSummary {
    PluginSummary(
      id: "notion",
      name: "notion",
      version: "1.0.0",
      displayName: "Notion",
      status: status,
      description: "Local Notion plugin",
      authorName: nil,
      enabled: true,
      defaultEnabled: true,
      capabilities: [
        "command:notion.run",
        "connector:notion",
        "skill:notion.notes",
        "mcp_server:notion",
      ],
      permissions: ["network.outbound"],
      manifestPath: "/tmp/notion/pith-plugin.json",
      provenance: "local",
      validationError: validationError,
      validationHint: validationHint
    )
  }

  private func pluginConnectorSummary(
    id: String,
    displayName: String,
    status: String,
    authStatus: String,
    credentialPresent: Bool
  ) -> PluginConnectorSummary {
    PluginConnectorSummary(
      id: id,
      displayName: displayName,
      service: "notion",
      pluginID: "notion",
      pluginDisplayName: "Notion",
      enabled: true,
      status: status,
      permissions: ["network.outbound"],
      manifestPath: "/tmp/notion/pith-plugin.json",
      homepage: nil,
      authType: "apiKey",
      authRequired: true,
      authScopes: ["pages"],
      credentialStore: "keychain",
      workflows: [],
      authStatus: authStatus,
      credentialPresent: credentialPresent,
      credentialSecretPresent: credentialPresent,
      credentialProvider: nil,
      credentialHandle: nil,
      credentialLabel: nil,
      authorizedAt: nil,
      credentialUpdatedAt: nil
    )
  }

  private func pluginCommandSummary(requiredConnectorIds: [String]) -> PluginCommandSummary {
    PluginCommandSummary(
      id: "notion.publish",
      title: "Publish Note",
      description: "Publish a local note.",
      pluginID: "notion",
      pluginDisplayName: "Notion",
      permissions: [],
      sourcePath: "/tmp/notion/pith-plugin.json",
      execution: nil,
      executionKind: nil,
      memorySummary: nil,
      runStatus: "needsConnectorAuth",
      runBlocker: nil,
      runRepairHint: nil,
      declaredConnectorIds: [],
      requiredConnectorIds: requiredConnectorIds,
      approvalRequired: false,
      approvalReason: nil
    )
  }
}

private struct TestPresentationError: LocalizedError {
  let message: String

  var errorDescription: String? {
    message
  }
}
