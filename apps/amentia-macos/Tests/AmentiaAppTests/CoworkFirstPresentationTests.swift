@testable import AmentiaApp
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
        dailyDriverNextAction: "Ask Amentia for the next cowork task."
      )
    )

    XCTAssertEqual(summary, "Ask Amentia for the next cowork task.")
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
      "Enable Web Search so Amentia can retrieve current information when needed."
    )
  }

  func testInspectorSessionUsesDailyDriverNextAction() {
    let summary = InspectorSessionPresenter.metaSummary(
      InspectorSessionSnapshot(
        runtimeState: .ready,
        isLocalModelReady: true,
        hasWorkspace: true,
        workspaceDisplayName: "Amentia",
        hasRuntimeThreadSelection: true,
        selectedThreadTitle: "Cowork",
        hasActiveTurn: false,
        setupReadyStepCount: SetupFlowState.stepCount,
        setupStepCount: SetupFlowState.stepCount,
        setupProgressDetail: "Ready",
        isWaitingForFirstMessage: false,
        runtimeReadinessStatus: "ready",
        dailyDriverStage: "ready",
        dailyDriverNextAction: "Ask Amentia for the next cowork task.",
        runtimeReadinessChecks: [],
        runtimeReadinessMetrics: localExecutionMetrics,
        selectedLocalExecutionSafetyMode: "askBeforeChange"
      )
    )

    XCTAssertTrue(summary.contains("Ask Amentia for the next cowork task."))
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

    XCTAssertEqual(runtimeStep?.label, "Amentia")
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

    XCTAssertEqual(detail, "Next: Refresh Model")
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
      error: RuntimeBridge.RuntimeError.rpc("backend stack trace")
    )

    XCTAssertEqual(entry.title, "Request Failed")
    XCTAssertTrue(entry.body.contains("prompt was restored"))
    XCTAssertFalse(entry.title.contains("Turn"))
    XCTAssertFalse(entry.body.contains("backend stack trace"))
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

  func testFirstRequestReadinessStepDraftsStarterPromptWhenEmpty() {
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

    XCTAssertEqual(
      RuntimeReadinessActionPlanner.title(for: action, snapshot: actionSnapshot),
      "Draft"
    )
    XCTAssertTrue(RuntimeReadinessActionPlanner.canRun(action, snapshot: actionSnapshot))
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

  func testComposerShowsModelSetupNextActionBeforeCoworkUnlocks() {
    let snapshot = ComposerStatusSnapshot(
      runtimeState: .ready,
      modelSetupTitle: "Almost Ready",
      modelSetupSummary: "Granite is selected. Amentia will run one final local check next.",
      modelSetupActionSummary: "Run the model check to unlock cowork.",
      isLocalModelReady: false,
      hasWorkspace: true,
      hasRuntimeThreadSelection: true,
      hasActiveTurn: false,
      isWaitingForFirstMessage: true,
      hasDraftMessage: false,
      hasRestoredLocalExecutionDraft: false
    )

    XCTAssertEqual(ComposerStatusPresenter.placeholder(snapshot), "Almost Ready")
    XCTAssertEqual(
      ComposerStatusPresenter.statusSummary(snapshot),
      "Granite is selected. Amentia will run one final local check next. Run the model check to unlock cowork."
    )
    XCTAssertFalse(ComposerStatusPresenter.statusSummary(snapshot).contains("Continue model setup"))
  }

  func testActiveWorkCopyAvoidsLocalExecutionTerminology() {
    let composer = composerSnapshot(hasDraftMessage: false, hasActiveTurn: true)
    let inspector = InspectorSessionSnapshot(
      runtimeState: .ready,
      isLocalModelReady: true,
      hasWorkspace: true,
      workspaceDisplayName: "Amentia",
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
      "Amentia is working. Cancel to stop it."
    )
    XCTAssertEqual(
      ComposerStatusPresenter.statusSummary(composer),
      "Amentia is working. Cancel the request if it is no longer useful."
    )
    XCTAssertEqual(InspectorSessionPresenter.title(inspector), "Amentia Is Working")
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
    XCTAssertFalse(error.localizedDescription.contains("AMENTIA_RUNTIME_PATH"))
    XCTAssertFalse(entry.body.contains("AMENTIA_RUNTIME_PATH"))
  }

  func testTimedOutRequestAvoidsRpcMethodNames() {
    let error = RuntimeBridge.RuntimeError.requestTimedOut(method: "turn/start", seconds: 210)

    XCTAssertTrue(error.localizedDescription.contains("current request"))
    XCTAssertTrue(error.localizedDescription.contains("210 seconds"))
    XCTAssertFalse(error.localizedDescription.contains("turn/start"))
    XCTAssertFalse(error.localizedDescription.contains("RPC"))
  }

  func testGenericRuntimeLaunchFailureUsesRecoveryCopy() {
    let error = RuntimeBridge.RuntimeError.rpc("JSON-RPC failed at /tmp/amentia-runtime")
    let detail = UserFacingFailurePresenter.runtimeLaunchFailureDetail(error: error)
    let entry = TimelineEventPresenter.runtimeLaunchFailed(error: error)

    XCTAssertTrue(detail.contains("Restart Amentia"))
    XCTAssertFalse(detail.contains("JSON-RPC"))
    XCTAssertFalse(detail.contains("/tmp"))
    XCTAssertEqual(entry.body, detail)
    XCTAssertEqual(entry.attributes["technicalError"], "JSON-RPC failed at /tmp/amentia-runtime")
  }

  func testCommonTimelineFailuresKeepRawErrorsOutOfBody() {
    let error = TestPresentationError(message: "backend failed at /Users/example/private.log")
    let entries = [
      TimelineEventPresenter.threadCreationFailed(error: error),
      TimelineEventPresenter.approvalResponseFailed(error: error),
      TimelineEventPresenter.turnCancelFailed(error: error),
      TimelineEventPresenter.threadLoadFailed(error: error),
      TimelineEventPresenter.workspaceOpenFailed(error: error),
      TimelineEventPresenter.memoryNoteFailed(error: error),
    ]

    for entry in entries {
      XCTAssertFalse(entry.body.contains("backend failed"))
      XCTAssertFalse(entry.body.contains("/Users/example"))
      XCTAssertEqual(entry.attributes["technicalError"], "backend failed at /Users/example/private.log")
    }
    XCTAssertFalse(
      TimelineEventPresenter.approvalResponseFailedDetail(error: error).contains("backend failed")
    )
    XCTAssertFalse(
      TimelineEventPresenter.turnCancelFailedDetail(error: error).contains("backend failed")
    )
  }

  func testProjectSearchFailureKeepsRawErrorsOutOfStatus() {
    let status = WorkspaceSearchSession.failureStatus(
      error: TestPresentationError(message: "ripgrep failed at /Users/example/project")
    )

    XCTAssertEqual(
      status,
      "Project search needs attention. Check Amentia status, then try again."
    )
    XCTAssertFalse(status.contains("/Users/example"))
    XCTAssertFalse(status.contains("ripgrep"))
  }

  func testFileRevealFailureCopyAvoidsRawPaths() {
    let detail = UserFacingFailurePresenter.fileRevealFailureDetail()

    XCTAssertTrue(detail.contains("Finder permissions"))
    XCTAssertFalse(detail.contains("/Users/example"))
    XCTAssertFalse(detail.contains("NSWorkspace"))
  }

  func testPluginFailureBodiesKeepRawErrorsInAttributesOnly() {
    let error = TestPresentationError(message: "runner failed at /Users/example/plugin.json")
    let entries = [
      TimelinePluginEventPresenter.pluginInstallPreviewFailed(
        error: error,
        repairHint: "Use a valid plugin folder.",
        sourcePath: "/Users/example/plugin"
      ),
      TimelinePluginEventPresenter.pluginInstallFailed(
        error: error,
        repairHint: "",
        sourcePath: "/Users/example/plugin"
      ),
      TimelinePluginEventPresenter.pluginUpdateFailed(
        pluginID: "notion",
        enabled: true,
        error: error
      ),
      TimelinePluginEventPresenter.pluginRemovalFailed(pluginID: "notion", error: error),
      TimelinePluginEventPresenter.pluginConnectorAuthFailed(
        connectorID: "notion::main",
        error: error
      ),
      TimelinePluginEventPresenter.pluginConnectorCredentialClearFailed(
        connectorID: "notion::main",
        error: error
      ),
      TimelinePluginEventPresenter.pluginCommandFailed(error: error),
    ]

    for entry in entries {
      XCTAssertFalse(entry.body.contains("/Users/example"))
      XCTAssertFalse(entry.body.contains("runner failed"))
      XCTAssertEqual(
        entry.attributes["technicalError"],
        "runner failed at /Users/example/plugin.json"
      )
    }
  }

  func testPluginRefreshDiagnosticsAvoidRawRegistryErrors() {
    let diagnostic = UserFacingFailurePresenter.pluginRefreshDiagnostic(
      label: "command registry"
    )

    XCTAssertEqual(diagnostic, "Plugin capabilities need attention.")
    XCTAssertFalse(diagnostic.contains("registry:"))
    XCTAssertFalse(diagnostic.contains("/Users/example"))
  }

  func testAppSupportFailureCopyAvoidsLocalPaths() {
    let detail = UserFacingFailurePresenter.appSupportDirectoryFailureDetail()

    XCTAssertTrue(detail.contains("local data folder"))
    XCTAssertFalse(detail.contains("/Users"))
    XCTAssertFalse(detail.contains("symbolic link"))
  }

  func testRequestPolicyMapsInternalMethodsToUserTasks() {
    XCTAssertEqual(
      RuntimeBridgeRequestPolicy.userFacingRequestName(for: "model/probe"),
      "model check"
    )
    XCTAssertEqual(
      RuntimeBridgeRequestPolicy.userFacingRequestName(for: "workspace/search"),
      "project search"
    )
    XCTAssertEqual(
      RuntimeBridgeRequestPolicy.userFacingRequestName(for: "plugin/connectorAuthorize"),
      "connection authorization"
    )
    XCTAssertEqual(
      RuntimeBridgeRequestPolicy.userFacingRequestName(for: "unknown/internal"),
      "current request"
    )
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
    let detail = PluginDashboardDetailPresenter.pluginDetailSummary(snapshot)
    let registry = PluginDashboardDetailPresenter.registryDetailSummary(snapshot)
    let permissions = PluginDashboardDetailPresenter.permissionDetailSummary(snapshot)

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

  func testPluginDashboardPreviewsStayBounded() {
    let snapshot = pluginDashboardSnapshot(
      capabilities: (0..<8).map { pluginCapabilitySummary(id: "capability-\($0)") },
      connectors: (0..<8).map {
        pluginConnectorSummary(
          id: "connection-\($0)",
          displayName: "Connection \($0)",
          status: "ready",
          authStatus: "ready",
          credentialPresent: false
        )
      },
      commands: (0..<8).map {
        pluginCommandSummary(id: "action-\($0)", requiredConnectorIds: [])
      },
      hooks: (0..<8).map { pluginHookSummary(id: "check-\($0)") }
    )

    XCTAssertEqual(PluginDashboardPreview.capabilityPreview(snapshot).count, 6)
    XCTAssertEqual(PluginDashboardPreview.connectorPreview(snapshot).count, 6)
    XCTAssertEqual(PluginDashboardPreview.commandPreview(snapshot).count, 6)
    XCTAssertEqual(PluginDashboardPreview.hookPreview(snapshot).count, 6)
    XCTAssertTrue(PluginDashboardPresenter.commandCountSummary(snapshot).contains("8 actions"))
    XCTAssertTrue(PluginDashboardPresenter.hookCountSummary(snapshot).contains("8 checks"))
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

    let connectorDetail = PluginDashboardDetailPresenter.connectorDetailSummary(snapshot)
    let commandDetail = PluginDashboardDetailPresenter.commandDetailSummary(snapshot)

    XCTAssertTrue(connectorDetail.contains("Authorization: needs sign in"))
    XCTAssertFalse(connectorDetail.contains("needsAuth"))
    XCTAssertTrue(commandDetail.contains("A required connection is missing."))
    XCTAssertFalse(commandDetail.contains("notion::missing"))
    XCTAssertFalse(commandDetail.contains("connectors"))
  }

  func testPluginDashboardDoesNotCountStaleConnectionMarkersAsAuthorized() {
    let snapshot = pluginDashboardSnapshot(
      connectors: [
        pluginConnectorSummary(
          id: "notion::main",
          displayName: "Notion",
          status: "needsAuth",
          authStatus: "needsAuth",
          credentialPresent: true
        )
      ]
    )

    let summary = PluginDashboardPresenter.connectorCountSummary(snapshot)

    XCTAssertEqual(summary, "1 connection | 1 need sign in")
    XCTAssertFalse(summary.contains("authorized"))
  }

  func testPluginDashboardDoesNotCountMissingTokensAsAuthorized() {
    let snapshot = pluginDashboardSnapshot(
      connectors: [
        pluginConnectorSummary(
          id: "notion::main",
          displayName: "Notion",
          status: "ready",
          authStatus: "authorized",
          credentialPresent: true,
          credentialSecretPresent: false
        )
      ]
    )

    let summary = PluginDashboardPresenter.connectorCountSummary(snapshot)
    let detail = PluginDashboardDetailPresenter.connectorDetailSummary(snapshot)

    XCTAssertEqual(summary, "1 connection | 1 ready | 1 need sign in")
    XCTAssertTrue(detail.contains("Authorization: needs sign in"))
    XCTAssertFalse(summary.contains("authorized"))
    XCTAssertFalse(detail.contains("authorized locally"))
  }

  func testPluginActionPlannerAllowsReauthorizationWhenTokenIsMissing() {
    let connector = missingTokenConnector()
    let snapshot = pluginActionSnapshot(connectors: [connector])

    XCTAssertTrue(
      PluginActionPlanner.canAuthorizeConnector(
        connectorID: "notion::main",
        snapshot: snapshot
      )
    )
    XCTAssertNil(
      PluginActionPlanner.connectorAuthorizeDisabledReason(
        connectorID: "notion::main",
        snapshot: snapshot
      )
    )
  }

  func testPluginActionPlannerFindsMissingTokenConnectorForAction() {
    let connector = missingTokenConnector()
    let command = connectorAuthCommand()
    let snapshot = pluginActionSnapshot(connectors: [connector], commands: [command])

    XCTAssertEqual(
      PluginActionPlanner.commandAuthorizationConnectorID(
        commandID: command.id,
        snapshot: snapshot
      ),
      "notion::main"
    )
  }

  func testPluginActionPlannerExplainsConnectorAuthorizationNextStep() {
    let connector = missingTokenConnector()
    let command = connectorAuthCommand()
    let snapshot = pluginActionSnapshot(connectors: [connector], commands: [command])

    let detail = PluginActionPlanner.commandConnectorAuthorizationDetail(
      commandID: command.id,
      snapshot: snapshot
    )

    XCTAssertEqual(
      detail,
      "Connect Notion before running Publish Note. Choose Authorize and paste a local token or API key."
    )
    XCTAssertEqual(
      PluginActionPlanner.commandRunDisabledReason(commandID: command.id, snapshot: snapshot),
      detail
    )
  }

  func testPluginCommandCoordinatorBlocksWithConnectorAuthorizationNextStep() {
    let connector = missingTokenConnector()
    let command = connectorAuthCommand()
    let snapshot = pluginActionSnapshot(connectors: [connector], commands: [command])

    let preparation = PluginCommandCoordinator.prepareRun(
      commandID: command.id,
      input: nil,
      selectedThreadID: "thread-1",
      snapshot: snapshot,
      commands: [command]
    )

    switch preparation {
    case let .blocked(blockedCommand, detail, input):
      XCTAssertEqual(blockedCommand.id, command.id)
      XCTAssertEqual(
        detail,
        "Connect Notion before running Publish Note. Choose Authorize and paste a local token or API key."
      )
      XCTAssertNil(input)
    case .ready, .unavailable:
      XCTFail("Expected the command to be blocked for connector authorization.")
    }
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

    let detail = PluginDashboardDetailPresenter.invalidPluginDetailSummary(snapshot)

    XCTAssertTrue(detail.contains("Setup needs review."))
    XCTAssertFalse(detail.contains("Unknown validation error"))
  }

  func testPluginValidationDetailHidesRawSetupPaths() {
    let plugin = pluginSummary(
      status: "invalid",
      validationError: "failed to read /tmp/notion/amentia-plugin.json: missing field displayName",
      validationHint: "Check /tmp/notion/amentia-plugin.json and use camelCase keys."
    )

    let detail = PluginStatusDisplay.validationDetail(plugin)

    XCTAssertTrue(detail.contains("Plugin setup needs review."))
    XCTAssertTrue(detail.contains("Fix: Check the plugin setup file, then refresh the plugin."))
    XCTAssertFalse(detail.contains("/tmp/notion"))
    XCTAssertFalse(detail.contains("amentia-plugin.json"))
    XCTAssertFalse(detail.contains("displayName"))
    XCTAssertFalse(detail.contains("camelCase"))
  }

  func testPluginDashboardSummaryHidesRawValidationHints() {
    let snapshot = pluginDashboardSnapshot(
      plugins: [
        pluginSummary(
          status: "invalid",
          validationError: "failed to parse /tmp/notion/amentia-plugin.json",
          validationHint: "Open /tmp/notion/amentia-plugin.json and fix displayName."
        )
      ]
    )

    let detail = PluginDashboardDetailPresenter.pluginDetailSummary(snapshot)

    XCTAssertTrue(detail.contains("needs attention"))
    XCTAssertTrue(detail.contains("Fix: Check the plugin setup file, then refresh the plugin."))
    XCTAssertFalse(detail.contains("/tmp/notion"))
    XCTAssertFalse(detail.contains("amentia-plugin.json"))
    XCTAssertFalse(detail.contains("displayName"))
  }

  func testPluginDashboardDetailsAvoidLogStyleLabels() {
    let command = pluginCommandSummary(
      requiredConnectorIds: [],
      execution: PluginCommandExecutionSummary(
        kind: "mcp.remote.createPage",
        driver: "node",
        entrypoint: nil,
        workflowID: nil,
        workflow: nil,
        input: PluginCommandEnvelopeSummary(envelope: "json", fields: [
          PluginCommandEnvelopeFieldSummary(
            name: "input",
            kind: "text",
            required: true,
            description: "Draft text"
          )
        ]),
        output: nil,
        supported: true
      )
    )
    let snapshot = pluginDashboardSnapshot(commands: [command])

    let detail = PluginDashboardDetailPresenter.commandDetailSummary(snapshot)

    XCTAssertTrue(detail.contains("Status:"))
    XCTAssertTrue(detail.contains("Input:"))
    XCTAssertFalse(detail.contains("status:"))
    XCTAssertFalse(detail.contains("input:"))
    XCTAssertFalse(detail.contains("blocked:"))
    XCTAssertFalse(detail.contains("fix:"))
  }

  func testPluginWorkflowSummaryAvoidsLogStyleLabels() {
    let execution = PluginCommandExecutionSummary(
      kind: "mcp.remote.createPage",
      driver: "node",
      entrypoint: nil,
      workflowID: "notion.create-page",
      workflow: PluginCommandWorkflowSummary(
        workflowID: "notion.create-page",
        displayName: "Create Page",
        connectorID: "notion::main",
        service: "notion",
        action: "createPage",
        maxAgentSteps: 2,
        stages: ["inspectBeforeWrite", "completed"],
        statuses: ["inspected", "success"],
        commandIDs: ["pages.create"]
      ),
      input: nil,
      output: nil,
      supported: true
    )

    let summary = PluginStatusDisplay.executionSummary(execution)

    XCTAssertTrue(summary.contains("Workflow ready"))
    XCTAssertFalse(summary.contains("workflow ready:"))
  }

  func testPluginCapabilityPresenterHidesRawMetadata() {
    let capability = PluginCapabilitySummary(
      id: "notion::mcp_server::notion-pages",
      kind: "mcp_server",
      identifier: "notion-pages",
      pluginID: "notion",
      pluginDisplayName: "Notion",
      permissions: ["network.outbound"],
      manifestPath: "/tmp/notion/amentia-plugin.json",
      metadata: [
        "definitionPath": "/tmp/notion/mcp/notion-pages.json",
        "definitionStatus": "missing",
        "definitionError": "failed to read /tmp/notion/mcp/notion-pages.json",
        "surface": "mcp_server",
      ]
    )

    let title = PluginCapabilityPresenter.title(capability)
    let review = PluginCapabilityPresenter.reviewSummary(capability) ?? ""
    let summary = PluginCapabilityPresenter.diagnosticSummary(capability) ?? ""
    let detail = PluginCapabilityPresenter.diagnosticDetail(capability) ?? ""
    let visibleText = [title, review, summary, detail].joined(separator: "\n")

    XCTAssertEqual(title, "MCP server")
    XCTAssertEqual(review, "MCP: local server, needs a local command.")
    XCTAssertTrue(detail.contains("plugin setup"))
    XCTAssertFalse(visibleText.contains("notion-pages"))
    XCTAssertFalse(visibleText.contains("/tmp/notion"))
    XCTAssertFalse(visibleText.contains("definitionPath"))
    XCTAssertFalse(visibleText.contains("mcp_server"))
  }

  func testPluginCapabilityReviewSummarizesConnectorWithoutRawMetadata() {
    let capability = PluginCapabilitySummary(
      id: "notion::connector:notion",
      kind: "connector",
      identifier: "notion",
      pluginID: "notion",
      pluginDisplayName: "Notion",
      permissions: ["network.outbound", "mcp.connect"],
      manifestPath: "/tmp/notion/amentia-plugin.json",
      metadata: [
        "surface": "connector",
        "displayName": "Notion",
        "service": "notion",
        "authType": "oauth2",
        "authRequired": "true",
        "authScopes": "read_content, insert_content",
        "credentialStore": "local",
        "homepage": "https://www.notion.so",
      ]
    )

    let title = PluginCapabilityPresenter.title(capability)
    let review = PluginCapabilityPresenter.reviewSummary(capability) ?? ""
    let visibleText = [title, review].joined(separator: "\n")

    XCTAssertEqual(title, "Connection: Notion")
    XCTAssertTrue(review.contains("Service: Notion"))
    XCTAssertTrue(review.contains("authorization required"))
    XCTAssertTrue(review.contains("auth: OAuth 2.0"))
    XCTAssertTrue(review.contains("access: read content, create content"))
    XCTAssertTrue(review.contains("token: stored locally"))
    XCTAssertFalse(visibleText.contains("connector:notion"))
    XCTAssertFalse(visibleText.contains("credentialStore"))
    XCTAssertFalse(visibleText.contains("secret:"))
    XCTAssertFalse(visibleText.contains("read_content"))
    XCTAssertFalse(visibleText.contains("https://www.notion.so"))
    XCTAssertFalse(visibleText.contains("/tmp/notion"))
  }

  func testPluginCapabilityReviewUsesSkillDescriptionButHidesPath() {
    let capability = PluginCapabilitySummary(
      id: "notes::skill:workspace",
      kind: "skill",
      identifier: "workspace",
      pluginID: "notes",
      pluginDisplayName: "Notes",
      permissions: [],
      manifestPath: "/tmp/notes/amentia-plugin.json",
      metadata: [
        "surface": "skill",
        "description": "Use project notes as bounded local guidance.",
        "path": "skills/workspace/SKILL.md",
      ]
    )

    let review = PluginCapabilityPresenter.reviewSummary(capability) ?? ""

    XCTAssertEqual(review, "Guidance: Use project notes as bounded local guidance.")
    XCTAssertFalse(review.contains("skills/workspace"))
    XCTAssertFalse(review.contains("SKILL.md"))
  }

  func testPluginSkillDisableCopyNamesGuidanceSource() {
    let skill = pluginSkillSummary(
      pluginDisplayName: "Workspace Notes",
      description: "Use project notes as bounded local guidance."
    )

    XCTAssertEqual(PluginSkillDisplay.disableButtonTitle(skill), "Disable Workspace Notes")
    XCTAssertEqual(
      PluginSkillDisplay.disableDetail(skill),
      "Stops Workspace Notes guidance from being added to future requests."
    )
  }

  func testPluginHookDisableCopyNamesCheckSource() {
    let hook = pluginHookSummary(id: "notion::hook:review")

    XCTAssertEqual(PluginHookDisplay.disableButtonTitle(hook), "Disable Notion")
    XCTAssertEqual(
      PluginHookDisplay.disableDetail(hook),
      "Stops Notion checks from running during future activity."
    )
  }

  func testPluginHookDisplayUsesProductEventLanguage() {
    let shellHook = pluginHookSummary(id: "shell::hook:recorder", event: "shell.completed")
    let unknownHook = pluginHookSummary(id: "custom::hook:opaque", event: "third_party.raw_event")

    XCTAssertEqual(PluginHookDisplay.eventDetail(shellHook), "Runs after shell commands")
    XCTAssertEqual(PluginHookDisplay.eventDetail(unknownHook), "Runs during plugin checks")
    XCTAssertEqual(
      PluginHookDisplay.statusLine(shellHook),
      "Notion | Runs after shell commands"
    )
    XCTAssertFalse(PluginHookDisplay.eventDetail(unknownHook).contains("third_party.raw_event"))
    XCTAssertFalse(PluginHookDisplay.statusLine(unknownHook).contains("third_party.raw_event"))
  }

  func testPluginDashboardHidesRawHookEvents() {
    let snapshot = pluginDashboardSnapshot(hooks: [
      pluginHookSummary(id: "shell::hook:recorder", event: "shell.completed"),
      pluginHookSummary(id: "custom::hook:opaque", event: "third_party.raw_event"),
    ])

    let detail = PluginDashboardDetailPresenter.hookDetailSummary(snapshot)

    XCTAssertTrue(detail.contains("Runs after shell commands"))
    XCTAssertTrue(detail.contains("Runs during plugin checks"))
    XCTAssertFalse(detail.contains("shell.completed"))
    XCTAssertFalse(detail.contains("third_party.raw_event"))
  }

  func testPluginCapabilityReviewSummarizesWorkflowWithoutRawIdentifiers() {
    let capability = PluginCapabilitySummary(
      id: "notion::connector_workflow:notion.create-page",
      kind: "connector_workflow",
      identifier: "notion.create-page",
      pluginID: "notion",
      pluginDisplayName: "Notion",
      permissions: ["network.outbound"],
      manifestPath: "/tmp/notion/amentia-plugin.json",
      metadata: [
        "surface": "connector_workflow",
        "displayName": "Publish Page",
        "connectorName": "Notion",
        "service": "notion",
        "action": "create_page",
        "maxAgentSteps": "4",
      ]
    )

    let title = PluginCapabilityPresenter.title(capability)
    let review = PluginCapabilityPresenter.reviewSummary(capability) ?? ""
    let visibleText = [title, review].joined(separator: "\n")

    XCTAssertEqual(title, "Workflow: Publish Page")
    XCTAssertEqual(review, "Workflow: service: Notion | action: create page | limit: up to 4 steps")
    XCTAssertFalse(visibleText.contains("notion.create-page"))
    XCTAssertFalse(visibleText.contains("connector_workflow"))
    XCTAssertFalse(visibleText.contains("/tmp/notion"))
  }

  func testUnknownPluginCapabilitiesAndPermissionsUseProductFallbacks() {
    let capabilitySummary = PluginCapabilityDisplay.summary([
      "command:run",
      "experimental_agent:planner",
    ])
    let unknownSurface = PluginCapabilityDisplay.surface("experimental_agent")
    let permissionSummary = PluginPermissionDisplay.summary([
      "network.outbound",
      "secrets.raw",
    ])

    XCTAssertEqual(capabilitySummary, "1 action | 1 capability")
    XCTAssertEqual(unknownSurface, "Capability")
    XCTAssertEqual(permissionSummary, "Custom local permission, Network access")
    XCTAssertFalse(capabilitySummary.contains("experimental_agent"))
    XCTAssertFalse(permissionSummary.contains("secrets.raw"))
  }

  func testPluginInstallConfirmationAvoidsRawPathsAndManifestTerms() {
    let preview = PluginInstallPreview(
      pluginID: "notion",
      sourcePath: "/Users/example/Desktop/notion-plugin",
      manifestPath: "/Users/example/Desktop/notion-plugin/amentia-plugin.json",
      installPath: "/Users/example/Library/Application Support/Amentia/plugins/notion",
      displayName: "Notion",
      version: "1.0.0",
      description: "Work with Notion locally.",
      authorName: "Amentia",
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
    XCTAssertTrue(text.contains("Installed: Stored in Amentia support data"))
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
      rootPath: "/Users/example/Projects/Amentia",
      displayName: "Amentia",
      threadCount: 1
    )
    let opened = TimelineEventPresenter.workspaceOpened(workspace)
    let restored = RuntimeLaunchAnnotationFactory.entries(RuntimeLaunchAnnotationSnapshot(
      serverName: "amentia-runtime",
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
    let restoreFailed = RuntimeLaunchAnnotationFactory.entries(RuntimeLaunchAnnotationSnapshot(
      serverName: "amentia-runtime",
      serverVersion: "0.1.0",
      shouldAnnotateSetupLaunch: false,
      restoredWorkspace: nil,
      skippedWorkspaceRestorePath: nil,
      workspaceRestoreErrorDetail: "open failed at /Users/example/private",
      modelHealth: nil,
      isLocalModelReady: false,
      localModelRequiredSummary: "Choose a local model to continue."
    )).first { $0.title == "Project Restore Failed" }
    let memory = TimelineEventPresenter.memoryNoteSaved(RuntimeBridge.RuntimeMemoryNote(
      id: "note-1",
      title: "Review habit",
      body: "Use concise project summaries.",
      scope: "Amentia",
      source: "workspace",
      createdAt: 1,
      tags: ["workspace"]
    ))

    XCTAssertEqual(opened.body, "Opened Amentia as the active project.")
    XCTAssertEqual(opened.attributes["workspacePath"], "/Users/example/Projects/Amentia")
    XCTAssertFalse(opened.body.contains("/Users/example"))
    XCTAssertEqual(restored?.body, "Restored Amentia as the active project.")
    XCTAssertFalse(restored?.body.contains("/Users/example") == true)
    XCTAssertEqual(
      restoreFailed?.body,
      "Could not restore the last project. Open a project folder to continue."
    )
    XCTAssertFalse(restoreFailed?.body.contains("/Users/example") == true)
    XCTAssertEqual(
      restoreFailed?.attributes["technicalError"],
      "open failed at /Users/example/private"
    )
    XCTAssertEqual(memory.body, "Saved project memory note Review habit.")
    XCTAssertFalse(memory.body.contains("built-in workspace"))
  }

  func testTimelineStatusCopyAvoidsInternalExecutionTerms() {
    let cancelled = TimelineEventPresenter.pendingTurnCancelled()

    XCTAssertEqual(TimelineEventPresenter.pendingTurnCancelledDetail, "Request cancelled.")
    XCTAssertEqual(TimelineEventPresenter.cancellingTurnDetail, "Cancelling request...")
    XCTAssertEqual(cancelled.body, "The pending request was cancelled before it finished.")
    XCTAssertEqual(
      TimelinePluginEventPresenter.pluginCommandNeedsExecutionContractDetail,
      "Plugin action needs a supported local runner before it can run."
    )
    XCTAssertFalse(cancelled.body.contains("local execution"))
    XCTAssertFalse(
      TimelinePluginEventPresenter.pluginCommandNeedsExecutionContractDetail.contains("contract")
    )
  }

  func testPluginInstallRepairHintsUseSetupLanguage() {
    XCTAssertEqual(
      PluginInstallDialogPresenter.repairHint(for: TestPresentationError(
        message: "Plugin does not contain amentia-plugin.json"
      )),
      "Choose a complete plugin folder, or select the plugin setup file directly."
    )
    XCTAssertEqual(
      PluginInstallDialogPresenter.repairHint(for: TestPresentationError(
        message: "Plugin cannot contain nested amentia-plugin.json manifests"
      )),
      "Remove nested plugin bundles before installing. Install each plugin as its own folder."
    )
    XCTAssertEqual(
      PluginInstallDialogPresenter.repairHint(for: TestPresentationError(
        message: "Select a plugin folder or the amentia-plugin.json manifest"
      )),
      "Point the installer at a plugin directory or the plugin setup file."
    )
  }

  func testPluginLifecycleTimelineUsesProductLanguage() {
    let preview = PluginInstallPreview(
      pluginID: "notion",
      sourcePath: "/Users/example/Desktop/notion-plugin",
      manifestPath: "/Users/example/Desktop/notion-plugin/amentia-plugin.json",
      installPath: "/Users/example/Library/Application Support/Amentia/plugins/notion",
      displayName: "Notion",
      version: "1.0.0",
      description: "Work with Notion locally.",
      authorName: "Amentia",
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
      authorName: "Amentia",
      enabled: true,
      defaultEnabled: true,
      capabilities: preview.capabilities,
      permissions: preview.permissions,
      manifestPath: preview.manifestPath,
      provenance: "local",
      validationError: nil,
      validationHint: nil
    )
    let installed = TimelinePluginEventPresenter.pluginInstalled(plugin, preview: preview)
    let removed = TimelinePluginEventPresenter.pluginRemoved(
      RuntimeBridge.RuntimePluginRemoval(
        pluginID: "notion",
        displayName: "Notion",
        removedPath: "/Users/example/Library/Application Support/Amentia/plugins/notion"
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

  func testConnectionReceiptUsesAuthorizationSummary() {
    let lines = TimelineConnectorReceiptPresenter.summaryLines(attributes: [
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

  func testConnectionReceiptDoesNotTreatStaleMarkerAsAuthorized() {
    let lines = TimelineConnectorReceiptPresenter.summaryLines(attributes: [
      "connectorId": "notion",
      "connectorService": "notion",
      "authStatus": "needsAuth",
      "credentialPresent": "true",
    ])

    XCTAssertEqual(
      lines.first,
      "Connection: Notion. Authorization: needs sign in."
    )
    XCTAssertFalse(lines.joined(separator: "\n").contains("saved locally"))
    XCTAssertFalse(lines.joined(separator: "\n").contains("authorized"))
  }

  func testConnectionReceiptDoesNotTreatMissingTokenAsAuthorized() {
    let lines = TimelineConnectorReceiptPresenter.summaryLines(attributes: [
      "connectorId": "notion",
      "connectorService": "notion",
      "credentialPresent": "true",
      "credentialSecretPresent": "false",
    ])

    XCTAssertEqual(
      lines.first,
      "Connection: Notion. Authorization: needs sign in."
    )
    XCTAssertFalse(lines.joined(separator: "\n").contains("saved locally"))
  }

  func testConnectionReceiptDoesNotAskForSignInWhenAuthorizationIsNotRequired() {
    let lines = TimelineConnectorReceiptPresenter.summaryLines(attributes: [
      "connectorId": "local-preview",
      "connectorService": "local",
      "authRequired": "false",
      "credentialPresent": "false",
      "credentialSecretPresent": "false",
    ])

    XCTAssertEqual(
      lines.first,
      "Connection: Local. Authorization: ready."
    )
    XCTAssertFalse(lines.joined(separator: "\n").contains("needs sign in"))
  }

  func testConnectionReceiptRequiresSignInWhenAuthorizationHasNoCredential() {
    let lines = TimelineConnectorReceiptPresenter.summaryLines(attributes: [
      "connectorId": "notion",
      "connectorService": "notion",
      "authRequired": "true",
      "credentialPresent": "false",
      "credentialSecretPresent": "false",
    ])

    XCTAssertEqual(
      lines.first,
      "Connection: Notion. Authorization: needs sign in."
    )
  }

  func testConnectionReceiptTreatsLocalBindingAsAvailable() {
    let lines = TimelineConnectorReceiptPresenter.summaryLines(attributes: [
      "connectorId": "notion",
      "connectorService": "notion",
      "authRequired": "true",
      "credentialPresent": "false",
      "credentialSecretPresent": "false",
      "credentialBinding": "env-bound",
    ])

    XCTAssertEqual(
      lines.first,
      "Connection: Notion. Authorization: available locally."
    )
    XCTAssertFalse(lines.joined(separator: "\n").contains("env-bound"))
    XCTAssertFalse(lines.joined(separator: "\n").contains("needs sign in"))
  }

  func testConnectionAuthorizationReceiptUsesProductCopy() {
    let connector = runtimePluginConnector(
      authStatus: "ready",
      credentialPresent: true,
      credentialSecretPresent: true
    )

    let entry = TimelinePluginEventPresenter.pluginConnectorAuthorized(connector)

    XCTAssertEqual(entry.title, "Connection Authorized")
    XCTAssertTrue(entry.body.contains("Notion is ready for Notion through Notion Connector."))
    XCTAssertTrue(entry.body.contains("Authorization: saved locally."))
    XCTAssertFalse(entry.body.contains("notion through"))
    XCTAssertFalse(entry.body.contains("keychain"))
    XCTAssertFalse(entry.body.contains("credentialStore"))
  }

  func testConnectionAuthorizationReceiptDoesNotTreatStaleMarkerAsAuthorized() {
    let connector = runtimePluginConnector(
      authStatus: "needsAuth",
      credentialPresent: true,
      credentialSecretPresent: false
    )

    let entry = TimelinePluginEventPresenter.pluginConnectorAuthorized(connector)

    XCTAssertTrue(entry.body.contains("Authorization: needs sign in."))
    XCTAssertEqual(entry.attributes["authorizationSummary"], "needs sign in")
    XCTAssertFalse(entry.body.contains("authorized without a secret"))
    XCTAssertFalse(entry.body.contains("saved locally"))
  }

  func testConnectionAuthorizationReceiptDoesNotTreatMissingTokenAsAuthorized() {
    let connector = runtimePluginConnector(
      authStatus: "ready",
      credentialPresent: true,
      credentialSecretPresent: false
    )

    let entry = TimelinePluginEventPresenter.pluginConnectorAuthorized(connector)

    XCTAssertTrue(entry.body.contains("Authorization: needs sign in."))
    XCTAssertEqual(entry.attributes["authorizationSummary"], "needs sign in")
    XCTAssertEqual(entry.attributes["credentialSecretPresent"], "false")
    XCTAssertFalse(entry.body.contains("authorized without a secret"))
  }

  func testAuthorizationStatusPrioritizesNeedsAuthOverStoredMarker() {
    XCTAssertEqual(
      PluginStatusDisplay.authorizationStatus(
        "needsAuth",
        authRequired: true,
        credentialPresent: true,
        credentialSecretPresent: false
      ),
      "needs sign in"
    )
    XCTAssertEqual(
      PluginStatusDisplay.authorizationStatus(
        "authorized",
        authRequired: true,
        credentialPresent: true,
        credentialSecretPresent: true
      ),
      "authorized locally"
    )
    XCTAssertEqual(
      PluginStatusDisplay.authorizationStatus(
        "authorized",
        authRequired: true,
        credentialPresent: true,
        credentialSecretPresent: false
      ),
      "needs sign in"
    )
    XCTAssertEqual(
      PluginStatusDisplay.authorizationStatus(
        "ready",
        authRequired: true,
        credentialPresent: false,
        credentialSecretPresent: false
      ),
      "needs sign in"
    )
    XCTAssertEqual(
      PluginStatusDisplay.authorizationStatus(
        "ready",
        authRequired: false,
        credentialPresent: false,
        credentialSecretPresent: false
      ),
      "ready"
    )
  }

  func testConnectionClearReceiptExplainsLocalCleanup() {
    let connector = runtimePluginConnector(
      authStatus: "needsAuth",
      credentialPresent: false,
      credentialSecretPresent: false
    )

    let entry = TimelinePluginEventPresenter.pluginConnectorCredentialCleared(connector)

    XCTAssertEqual(entry.title, "Connection Authorization Cleared")
    XCTAssertTrue(entry.body.contains("Notion authorization for Notion was cleared"))
    XCTAssertTrue(entry.body.contains("Any saved local token or key is no longer available"))
    XCTAssertFalse(entry.body.contains("local plugin state"))
    XCTAssertFalse(entry.body.contains("keychain"))
    XCTAssertFalse(entry.body.contains("credentialStore"))
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
      modelSetupActionSummary: "Continue setup.",
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
      runtimeLaunchButtonTitle: "Start Amentia",
      modelSetupActionTitle: nil
    )
  }

  private func readinessSnapshotForFirstRequest() -> RuntimeReadinessSnapshot {
    RuntimeReadinessSnapshot(
      runtimeState: .ready,
      modelReadinessDetail: "Ready",
      modelTone: .ready,
      workspaceDisplayName: "Amentia",
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
    "amentiaAccountRequired": "false",
    "defaultLocalExecutionSafetyMode": "askBeforeChange",
    "localExecutionSafetyModes": "explore,askBeforeChange,approvedWorkspaceExecution",
  ]

  private func pluginDashboardSnapshot(
    plugins: [PluginSummary]? = nil,
    capabilities: [PluginCapabilitySummary] = [],
    connectors: [PluginConnectorSummary] = [],
    commands: [PluginCommandSummary] = [],
    hooks: [PluginHookSummary] = [],
    skills: [PluginSkillSummary] = []
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
      capabilities: capabilities,
      connectors: connectors,
      commands: commands,
      hooks: hooks,
      skills: skills,
      diagnostics: [],
      refreshRecoveryAttributes: [:],
      hasLifecycleOperation: false
    )
  }

  private func pluginActionSnapshot(
    connectors: [PluginConnectorSummary],
    commands: [PluginCommandSummary] = []
  ) -> PluginActionSnapshot {
    PluginActionSnapshot(
      runtimeState: .ready,
      hasRuntimeThreadSelection: true,
      selectedThreadID: "thread-1",
      hasActiveOrPendingTurn: false,
      hasLifecycleOperation: false,
      plugins: [pluginSummary()],
      connectors: connectors,
      commands: commands
    )
  }

  private func pluginCapabilitySummary(id: String) -> PluginCapabilitySummary {
    PluginCapabilitySummary(
      id: id,
      kind: "skill",
      identifier: id,
      pluginID: "notion",
      pluginDisplayName: "Notion",
      permissions: [],
      manifestPath: "/tmp/notion/amentia-plugin.json",
      metadata: ["description": "Bounded local guidance."]
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
      manifestPath: "/tmp/notion/amentia-plugin.json",
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
    credentialPresent: Bool,
    credentialSecretPresent: Bool? = nil
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
      manifestPath: "/tmp/notion/amentia-plugin.json",
      homepage: nil,
      authType: "apiKey",
      authRequired: true,
      authScopes: ["pages"],
      credentialStore: "keychain",
      workflows: [],
      authStatus: authStatus,
      credentialPresent: credentialPresent,
      credentialSecretPresent: credentialSecretPresent ?? credentialPresent,
      credentialProvider: nil,
      credentialHandle: nil,
      credentialLabel: nil,
      authorizedAt: nil,
      credentialUpdatedAt: nil
    )
  }

  private func missingTokenConnector(id: String = "notion::main") -> PluginConnectorSummary {
    pluginConnectorSummary(
      id: id,
      displayName: "Notion",
      status: "ready",
      authStatus: "authorized",
      credentialPresent: true,
      credentialSecretPresent: false
    )
  }

  private func runtimePluginConnector(
    authStatus: String,
    credentialPresent: Bool,
    credentialSecretPresent: Bool
  ) -> RuntimeBridge.RuntimePluginConnector {
    RuntimeBridge.RuntimePluginConnector(
      connectorID: "notion-connector::notion",
      displayName: "Notion",
      service: "notion",
      pluginID: "notion-connector",
      pluginDisplayName: "Notion Connector",
      enabled: true,
      status: "ready",
      permissions: ["network.outbound", "mcp.connect"],
      manifestPath: "/plugins/notion-connector/amentia-plugin.json",
      homepage: "https://www.notion.so",
      authType: "api_key",
      authRequired: true,
      authScopes: ["read_content", "insert_content"],
      credentialStore: "keychain",
      workflows: [],
      authStatus: authStatus,
      credentialPresent: credentialPresent,
      credentialSecretPresent: credentialSecretPresent,
      credentialProvider: nil,
      credentialHandle: nil,
      credentialLabel: "Local Notion integration token",
      authorizedAt: 1_700_000_000,
      credentialUpdatedAt: 1_700_000_000
    )
  }

  private func pluginCommandSummary(
    id: String = "notion.publish",
    requiredConnectorIds: [String],
    execution: PluginCommandExecutionSummary? = nil
  ) -> PluginCommandSummary {
    PluginCommandSummary(
      id: id,
      title: "Publish Note",
      description: "Publish a local note.",
      pluginID: "notion",
      pluginDisplayName: "Notion",
      permissions: [],
      sourcePath: "/tmp/notion/amentia-plugin.json",
      execution: execution,
      executionKind: execution?.kind,
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

  private func supportedPluginExecution() -> PluginCommandExecutionSummary {
    PluginCommandExecutionSummary(
      kind: "mcp.remote.createPage",
      driver: "node",
      entrypoint: nil,
      workflowID: "notion.create-page",
      workflow: nil,
      input: nil,
      output: nil,
      supported: true
    )
  }

  private func connectorAuthCommand(
    requiredConnectorIds: [String] = ["notion::main"]
  ) -> PluginCommandSummary {
    pluginCommandSummary(
      requiredConnectorIds: requiredConnectorIds,
      execution: supportedPluginExecution()
    )
  }

  private func pluginSkillSummary(
    pluginDisplayName: String = "Notion",
    description: String = "Use bounded local guidance."
  ) -> PluginSkillSummary {
    PluginSkillSummary(
      id: "notion::skill:notes",
      description: description,
      pluginID: "notion",
      pluginDisplayName: pluginDisplayName,
      permissions: [],
      sourcePath: "/tmp/notion/skills/notes/SKILL.md",
      status: "ready",
      preview: "Keep guidance bounded.",
      contentBytes: 128,
      runBlocker: nil,
      runRepairHint: nil
    )
  }

  private func pluginHookSummary(
    id: String,
    event: String = "after_action"
  ) -> PluginHookSummary {
    PluginHookSummary(
      id: id,
      title: "Review Output",
      description: "Check plugin output before presenting it.",
      event: event,
      pluginID: "notion",
      pluginDisplayName: "Notion",
      permissions: [],
      sourcePath: "/tmp/notion/amentia-plugin.json",
      status: "ready",
      runBlocker: nil,
      runRepairHint: nil,
      memorySummary: nil
    )
  }
}

private struct TestPresentationError: LocalizedError {
  let message: String

  var errorDescription: String? {
    message
  }
}
