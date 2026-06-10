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
        detail: "No plugin command capabilities are required."
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
      hasActiveTurn: false,
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
}
