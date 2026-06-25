@testable import AmentiaApp
import XCTest

final class LocalModelProbeCoordinatorTests: XCTestCase {
  func testPostActivationCheckWaitsUntilModelCanBeProbed() {
    let coordinator = LocalModelProbeCoordinator()
    coordinator.schedulePostActivationCheck(modelID: "granite-4.0-h-350m")

    XCTAssertNil(coordinator.consumePostActivationCheck(
      activeModelID: "granite-4.0-h-350m",
      canProbe: false
    ))

    XCTAssertEqual(
      coordinator.consumePostActivationCheck(
        activeModelID: "granite-4.0-h-350m",
        canProbe: true
      ),
      LocalModelProbeRequest(modelID: "granite-4.0-h-350m")
    )
  }

  func testPostActivationCheckClearsWhenActiveModelChanges() {
    let coordinator = LocalModelProbeCoordinator()
    coordinator.schedulePostActivationCheck(modelID: "granite-4.0-h-350m")

    XCTAssertNil(coordinator.consumePostActivationCheck(
      activeModelID: "minicpm5-1b",
      canProbe: true
    ))
    XCTAssertNil(coordinator.consumePostActivationCheck(
      activeModelID: "granite-4.0-h-350m",
      canProbe: true
    ))
  }

  func testPostActivationCheckCanBeCancelled() {
    let coordinator = LocalModelProbeCoordinator()
    coordinator.schedulePostActivationCheck(modelID: "granite-4.0-h-350m")
    coordinator.cancelPendingPostActivationCheck()

    XCTAssertNil(coordinator.consumePostActivationCheck(
      activeModelID: "granite-4.0-h-350m",
      canProbe: true
    ))
  }
}
