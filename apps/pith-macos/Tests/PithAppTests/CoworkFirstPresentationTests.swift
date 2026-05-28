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
        runtimeReadinessChecks: []
      )
    )

    XCTAssertTrue(summary.contains("Ask Pith for the next cowork task."))
  }

  func testComposerFramesDraftAsCoworkPrompt() {
    let placeholder = ComposerStatusPresenter.placeholder(composerSnapshot(hasDraftMessage: true))
    let summary = ComposerStatusPresenter.statusSummary(composerSnapshot(hasDraftMessage: true))

    XCTAssertEqual(placeholder, "Review the first cowork prompt, then send")
    XCTAssertEqual(summary, "Review the starter prompt, then start the cowork session.")
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

  private func composerSnapshot(hasDraftMessage: Bool) -> ComposerStatusSnapshot {
    ComposerStatusSnapshot(
      runtimeState: .ready,
      modelSetupTitle: "Download model",
      modelSetupSummary: "Model ready.",
      isLocalModelReady: true,
      hasWorkspace: true,
      hasRuntimeThreadSelection: true,
      hasActiveTurn: false,
      isWaitingForFirstMessage: true,
      hasDraftMessage: hasDraftMessage
    )
  }
}
