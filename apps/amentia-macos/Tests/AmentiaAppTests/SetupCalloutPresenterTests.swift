@testable import AmentiaApp
import XCTest

final class SetupCalloutPresenterTests: XCTestCase {
  func testDisconnectedRuntimeIncludesAdHocTrustRepairHint() {
    let detail = SetupCalloutPresenter.detail(
      snapshot(runtimeState: .disconnected, distributionTrustSetupDetail: "Installer trust: Open Anyway.")
    )

    XCTAssertTrue(detail.contains("local on this Mac"))
    XCTAssertTrue(detail.contains("Open Anyway"))
  }

  func testReadyRuntimeKeepsModelDetailFocused() {
    let detail = SetupCalloutPresenter.detail(
      snapshot(runtimeState: .ready, distributionTrustSetupDetail: "Installer trust: Open Anyway.")
    )

    XCTAssertTrue(detail.contains("local on this Mac"))
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
        title: "Start Amentia",
        summary: "Start Amentia before choosing or running a model.",
        detail: "The model catalog, downloads, and selected model state stay local on this Mac.",
        actionSummary: "Start Amentia to inspect model setup.",
        readinessDetail: "Launch",
        tone: .warning
      ),
      modelProgressDetail: nil,
      runtimeLaunchActionTitle: "Start Amentia",
      modelPrimaryActionTitle: nil,
      modelSecondaryActionTitle: nil,
      distributionTrustSetupDetail: distributionTrustSetupDetail
    )
  }
}
