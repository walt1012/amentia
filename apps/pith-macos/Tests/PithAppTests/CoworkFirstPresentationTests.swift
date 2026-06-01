@testable import PithApp
import XCTest

final class CoworkFirstPresentationTests: XCTestCase {
  func testRuntimeHeaderFramesFirstUseAsCoworkSession() {
    let summary = RuntimeHeaderPresenter.statusSummary(headerSnapshot(hasDraftMessage: false))

    XCTAssertEqual(summary, "Ready to start the first cowork session.")
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

    XCTAssertEqual(placeholder, "Review the restored request, then send")
    XCTAssertEqual(summary, "Ask mode is ready. Review the restored request, then send it.")
  }

  func testFirstReadyTimelineEventFramesCoworkRequest() {
    let entry = TimelineEventPresenter.firstRequestReady()

    XCTAssertEqual(entry.title, "Cowork Session Ready")
    XCTAssertTrue(entry.body.contains("cowork request"))
  }

  private func headerSnapshot(
    hasDraftMessage: Bool,
    isWaitingForFirstMessage: Bool = true,
    dailyDriverStage: String? = nil,
    dailyDriverNextAction: String? = nil
  ) -> RuntimeHeaderSnapshot {
    RuntimeHeaderSnapshot(
      runtimeState: .ready,
      runtimeDetail: "",
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

  private let localExecutionMetrics = [
    "pithAccountRequired": "false",
    "defaultLocalExecutionSafetyMode": "askBeforeChange",
    "localExecutionSafetyModes": "explore,askBeforeChange,approvedWorkspaceExecution",
  ]
}
