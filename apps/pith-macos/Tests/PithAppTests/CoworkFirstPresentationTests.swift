@testable import PithApp
import XCTest

final class CoworkFirstPresentationTests: XCTestCase {
  func testRuntimeHeaderFramesFirstUseAsCoworkSession() {
    let summary = RuntimeHeaderPresenter.statusSummary(headerSnapshot(hasDraftMessage: false))

    XCTAssertEqual(
      summary,
      "Choose Understand Project, Pick Next Step, or type a short cowork prompt."
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

  func testRuntimeReadinessStepUsesLocalServiceLanguage() {
    let runtimeStep = RuntimeReadinessPresenter.steps(readinessSnapshotForFirstRequest())
      .first { $0.id == "runtime" }

    XCTAssertEqual(runtimeStep?.label, "Service")
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
        title: "Connectors",
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
      detail: "Connectors Setup",
      tone: .warning
    )
    let snapshot = readinessActionSnapshot(checks: [
      RuntimeReadinessCheckSummary(
        id: "plugins",
        title: "Connectors",
        status: "setup_required",
        detail: "Enable command capability"
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
    hasRestoredLocalExecutionDraft: Bool = false
  ) -> ComposerStatusSnapshot {
    ComposerStatusSnapshot(
      runtimeState: .ready,
      modelSetupTitle: "Download model",
      modelSetupSummary: "Model ready.",
      isLocalModelReady: true,
      hasWorkspace: true,
      hasRuntimeThreadSelection: true,
      hasActiveTurn: false,
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
        title: "Execution Controls",
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
      runtimeLaunchButtonTitle: "Start Local Service",
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

  private func pluginDashboardSnapshot() -> PluginDashboardSnapshot {
    PluginDashboardSnapshot(
      plugins: [
        PluginSummary(
          id: "notion",
          name: "notion",
          version: "1.0.0",
          displayName: "Notion",
          status: "ready",
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
          validationError: nil,
          validationHint: nil
        )
      ],
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
      connectors: [],
      commands: [],
      hooks: [],
      diagnostics: [],
      refreshRecoveryAttributes: [:],
      hasLifecycleOperation: false
    )
  }
}
