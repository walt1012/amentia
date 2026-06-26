@testable import AmentiaApp
import XCTest

final class LocalModelReadinessStateTests: XCTestCase {
  func testFailedProbeBlocksOnlyMatchingActiveModel() {
    var state = LocalModelReadinessState(
      models: [
        model(id: "granite-4.0-h-350m", active: true),
        model(id: "minicpm5-1b", active: false),
      ],
      selectedSetupModelID: "granite-4.0-h-350m"
    )

    state.applyProbeResult(
      modelID: "granite-4.0-h-350m",
      status: "error",
      detail: "llama backend failed at /Users/example/model.gguf"
    )

    XCTAssertTrue(state.blocksReadiness(activeModelID: "granite-4.0-h-350m"))
    XCTAssertFalse(state.blocksReadiness(activeModelID: "minicpm5-1b"))
    XCTAssertEqual(
      state.probeFailureDetail(activeModelID: "granite-4.0-h-350m"),
      "Cowork is paused until the local model starts successfully. Restart Amentia or re-download the selected model."
    )
    XCTAssertFalse(
      state.probeFailureDetail(activeModelID: "granite-4.0-h-350m")?.contains("/Users/example")
        == true
    )
    XCTAssertFalse(
      state.probeFailureDetail(activeModelID: "granite-4.0-h-350m")?.contains("llama")
        == true
    )
  }

  func testCatalogRefreshClearsProbeStateWhenActiveModelChanges() {
    var state = LocalModelReadinessState(
      models: [
        model(id: "granite-4.0-h-350m", active: true),
        model(id: "minicpm5-1b", active: false),
      ],
      selectedSetupModelID: "granite-4.0-h-350m"
    )
    state.markProbeStarted(modelID: "granite-4.0-h-350m")

    state.applyCatalogRefresh(LocalModelCatalogRefreshPlan(
      models: [
        model(id: "granite-4.0-h-350m", active: false),
        model(id: "minicpm5-1b", active: true),
      ],
      selectedSetupModelID: "minicpm5-1b",
      shouldClearConfiguredActiveModel: false
    ))

    XCTAssertFalse(state.blocksReadiness(activeModelID: "minicpm5-1b"))
    XCTAssertNil(state.probeFailureDetail(activeModelID: "granite-4.0-h-350m"))
  }

  private func model(id: String, active: Bool) -> LocalModelSummary {
    LocalModelSummary(
      id: id,
      displayName: id,
      description: "Local model fixture.",
      fileName: "\(id).gguf",
      downloadURL: "https://example.com/\(id).gguf",
      homepage: "https://example.com/\(id)",
      sizeBytes: 1,
      sha256: String(repeating: "a", count: 64),
      contextSize: 4096,
      modelContextSize: 4096,
      maxOutputTokens: 192,
      license: "apache-2.0",
      tags: ["test"],
      installPath: "/tmp/\(id).gguf",
      downloaded: true,
      active: active,
      localSizeBytes: 1
    )
  }
}
