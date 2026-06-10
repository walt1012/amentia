@testable import PithApp
import XCTest

final class LocalModelPresenterTests: XCTestCase {
  func testDefaultDownloadButtonNamesSelectedFirstUseModel() {
    let snapshot = statusSnapshot(selectedModel: model(
      id: "lfm2.5-350m",
      displayName: "LFM2.5-350M Q4_K_M",
      downloaded: false,
      active: false
    ))

    XCTAssertEqual(
      LocalModelStatusPresenter.defaultDownloadButtonTitle(snapshot),
      "Download LFM2.5-350M"
    )
  }

  func testDownloadedModelActionNamesSelectedModel() {
    let snapshot = statusSnapshot(selectedModel: model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: true,
      active: false
    ))

    XCTAssertEqual(
      LocalModelStatusPresenter.defaultDownloadButtonTitle(snapshot),
      "Use Granite 4.0-H-350M"
    )
  }

  func testPausedDownloadActionNamesSelectedModel() {
    let selectedModel = model(
      id: "lfm2.5-350m",
      displayName: "LFM2.5-350M Q4_K_M",
      downloaded: false,
      active: false
    )
    let snapshot = statusSnapshot(
      selectedModel: selectedModel,
      pausedModelDownloadID: selectedModel.id
    )

    XCTAssertEqual(
      LocalModelStatusPresenter.defaultDownloadButtonTitle(snapshot),
      "Continue LFM2.5-350M"
    )
  }

  func testRecoverySummaryExplainsPausedDownloadChoices() {
    let selectedModel = model(
      id: "lfm2.5-350m",
      displayName: "LFM2.5-350M Q4_K_M",
      downloaded: false,
      active: false
    )
    let summary = LocalModelOperationPresenter.recoverySummary(
      operationSnapshot(pausedModel: selectedModel, selectedModel: selectedModel)
    )

    XCTAssertTrue(summary.contains("Continue LFM2.5-350M"))
    XCTAssertFalse(summary.contains("Q4_K_M"))
    XCTAssertTrue(summary.contains("cancel to remove the partial file"))
  }

  func testRecoverySummaryExplainsDownloadedModelRepair() {
    let selectedModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: true,
      active: false
    )
    let summary = LocalModelOperationPresenter.recoverySummary(
      operationSnapshot(selectedModel: selectedModel)
    )

    XCTAssertTrue(summary.contains("Use Granite 4.0-H-350M"))
    XCTAssertFalse(summary.contains("Q4_K_M"))
    XCTAssertTrue(summary.contains("Refresh local model setup"))
    XCTAssertFalse(summary.contains("setup info"))
  }

  func testSetupGuidanceNamesModelDownloadInsteadOfEngineDownload() {
    let selectedModel = model(
      id: "lfm2.5-350m",
      displayName: "LFM2.5-350M Q4_K_M",
      downloaded: false,
      active: false
    )
    let guidance = LocalModelOperationPresenter.setupGuidance(
      operationSnapshot(selectedModel: selectedModel)
    )

    XCTAssertEqual(guidance.title, "Download Local Model")
    XCTAssertTrue(guidance.detail.contains("Size"))
    XCTAssertFalse(guidance.title.contains("Engine"))
    XCTAssertFalse(guidance.detail.contains("|"))
  }

  func testSetupModelChoiceDetailAvoidsTechnicalSeparators() {
    let selectedModel = model(
      id: "lfm2.5-350m",
      displayName: "LFM2.5-350M Q4_K_M",
      downloaded: false,
      active: false
    )
    let detail = LocalModelOperationPresenter.setupModelChoiceDetail(
      operationSnapshot(selectedModel: selectedModel),
      defaultModelID: selectedModel.id
    )

    XCTAssertTrue(detail.contains("LFM2.5-350M"))
    XCTAssertTrue(detail.contains("one verified local model"))
    XCTAssertFalse(detail.contains("|"))
    XCTAssertFalse(detail.contains("Q4_K_M"))
  }

  func testLocalModelStatusSummaryAvoidsPipeSeparators() {
    let selectedModel = model(
      id: "lfm2.5-350m",
      displayName: "LFM2.5-350M Q4_K_M",
      downloaded: true,
      active: true
    )
    let summary = LocalModelStatusPresenter.localModelStatusSummary(
      selectedModel,
      snapshot: statusSnapshot(selectedModel: selectedModel)
    )

    XCTAssertTrue(summary.contains("active"))
    XCTAssertTrue(summary.contains("License"))
    XCTAssertFalse(summary.contains("|"))
  }

  func testDownloadProgressSummaryAvoidsPipeSeparators() {
    let selectedModel = model(
      id: "lfm2.5-350m",
      displayName: "LFM2.5-350M Q4_K_M",
      downloaded: false,
      active: false
    )
    let startedAt = Date(timeIntervalSince1970: 1)
    let summary = LocalModelDownloadStatusPresenter.downloadProgressSummary(
      statusSnapshot(
        selectedModel: selectedModel,
        modelDownloadID: selectedModel.id,
        progress: ModelDownloadProgress(
          modelID: selectedModel.id,
          displayName: selectedModel.displayName,
          bytesReceived: 50,
          totalBytes: 100,
          startedAt: startedAt,
          updatedAt: startedAt.addingTimeInterval(10),
          isResuming: false
        )
      )
    )

    XCTAssertTrue(summary.contains("50% complete"))
    XCTAssertFalse(summary.contains("Q4_K_M"))
    XCTAssertFalse(summary.contains("|"))
  }

  func testDownloadInterruptionCopyUsesFriendlyModelName() {
    let selectedModel = model(
      id: "lfm2.5-350m",
      displayName: "LFM2.5-350M Q4_K_M",
      downloaded: false,
      active: false
    )
    let plan = LocalModelDownloadInterruptionPlanner.cancellationPlan(model: selectedModel)

    XCTAssertEqual(plan.timelineTitle, "Model Download Cancelled")
    XCTAssertTrue(plan.runtimeDetail.contains("LFM2.5-350M"))
    XCTAssertFalse(plan.runtimeDetail.contains("Q4_K_M"))
    XCTAssertFalse(plan.timelineTitle.contains("Engine"))
  }

  func testRepairGuidanceAvoidsInternalSetupInfoLanguage() {
    let guidance = LocalModelOperationPresenter.setupGuidance(operationSnapshot())

    XCTAssertEqual(guidance.title, "Repair Model Setup")
    XCTAssertTrue(guidance.summary.contains("setup is refreshed"))
    XCTAssertFalse(guidance.detail.contains("setup info"))
  }

  func testRecoverySummaryExplainsRuntimeRelaunch() {
    let summary = LocalModelOperationPresenter.recoverySummary(
      operationSnapshot(runtimeState: .failed)
    )

    XCTAssertTrue(summary.contains("Restart the local service"))
    XCTAssertTrue(summary.contains("selected model choices remain local"))
  }

  private func statusSnapshot(
    selectedModel: LocalModelSummary,
    modelDownloadID: String? = nil,
    pausedModelDownloadID: String? = nil,
    progress: ModelDownloadProgress? = nil
  ) -> LocalModelStatusSnapshot {
    LocalModelStatusSnapshot(
      runtimeState: .ready,
      modelHealth: nil,
      modelDownloadID: modelDownloadID,
      pausedModelDownloadID: pausedModelDownloadID,
      modelDownloadProgress: progress,
      selectedSetupModelID: selectedModel.id,
      selectedSetupModel: selectedModel,
      hasActiveCatalogModel: false
    )
  }

  private func operationSnapshot(
    runtimeState: RuntimeBridge.ConnectionState = .ready,
    isLocalModelReady: Bool = false,
    hasActiveTurn: Bool = false,
    downloadingModel: LocalModelSummary? = nil,
    pausedModel: LocalModelSummary? = nil,
    selectedModel: LocalModelSummary? = nil,
    selectedDownloadBlockedDetail: String? = nil,
    downloadedModelCount: Int = 0,
    totalModelCount: Int = 2,
    activeModelDisplayName: String? = nil,
    downloadedLocalSizeBytes: Int64 = 0
  ) -> LocalModelOperationSnapshot {
    LocalModelOperationSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady,
      hasActiveTurn: hasActiveTurn,
      downloadingModel: downloadingModel,
      pausedModel: pausedModel,
      selectedSetupModel: selectedModel,
      selectedDownloadBlockedDetail: selectedDownloadBlockedDetail,
      downloadedModelCount: downloadedModelCount,
      totalModelCount: totalModelCount,
      activeModelDisplayName: activeModelDisplayName,
      downloadedLocalSizeBytes: downloadedLocalSizeBytes
    )
  }

  private func model(
    id: String,
    displayName: String,
    downloaded: Bool,
    active: Bool
  ) -> LocalModelSummary {
    LocalModelSummary(
      id: id,
      displayName: displayName,
      description: "Tiny local model.",
      fileName: "\(id).gguf",
      downloadURL: "https://example.com/\(id).gguf",
      homepage: "https://example.com/\(id)",
      sizeBytes: 222_000_000,
      sha256: String(repeating: "a", count: 64),
      contextSize: 4096,
      modelContextSize: 32_768,
      maxOutputTokens: 160,
      license: "apache-2.0",
      tags: ["tiny"],
      installPath: "/tmp/\(id).gguf",
      downloaded: downloaded,
      active: active,
      localSizeBytes: downloaded ? 222_000_000 : nil
    )
  }
}
