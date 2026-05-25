@testable import PithApp
import XCTest

final class SetupCalloutPresenterTests: XCTestCase {
  func testDisconnectedRuntimeIncludesAdHocTrustRepairHint() {
    let detail = SetupCalloutPresenter.detail(
      snapshot(runtimeState: .disconnected, distributionTrustSetupDetail: "Installer trust: Open Anyway.")
    )

    XCTAssertTrue(detail.contains("local runtime"))
    XCTAssertTrue(detail.contains("Open Anyway"))
  }

  func testReadyRuntimeKeepsModelDetailFocused() {
    let detail = SetupCalloutPresenter.detail(
      snapshot(runtimeState: .ready, distributionTrustSetupDetail: "Installer trust: Open Anyway.")
    )

    XCTAssertTrue(detail.contains("local runtime"))
    XCTAssertFalse(detail.contains("Open Anyway"))
  }

  private func snapshot(
    runtimeState: RuntimeBridge.ConnectionState,
    distributionTrustSetupDetail: String?
  ) -> SetupCalloutSnapshot {
    SetupCalloutSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: false,
      hasWorkspace: false,
      hasRuntimeThreadSelection: false,
      modelGuidance: LocalModelSetupGuidance(
        title: "Launch Local Runtime",
        summary: "Launch Pith's local runtime before choosing or running a model.",
        detail: "The model catalog, downloads, and active model state stay inside the local runtime.",
        actionSummary: "Launch the runtime to inspect local model setup.",
        readinessDetail: "Launch",
        tone: .warning
      ),
      modelProgressDetail: nil,
      runtimeLaunchActionTitle: "Launch Runtime",
      modelPrimaryActionTitle: nil,
      modelSecondaryActionTitle: nil,
      distributionTrustSetupDetail: distributionTrustSetupDetail
    )
  }
}
