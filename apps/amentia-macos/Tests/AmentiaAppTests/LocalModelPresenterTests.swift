@testable import AmentiaApp
import XCTest

final class LocalModelPresenterTests: XCTestCase {
  func testDefaultDownloadButtonNamesSelectedFirstUseModel() {
    let snapshot = statusSnapshot(selectedModel: model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: false,
      active: false
    ))

    XCTAssertEqual(
      LocalModelStatusPresenter.defaultDownloadButtonTitle(snapshot),
      "Download Granite 4.0-H-350M"
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
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: false,
      active: false
    )
    let snapshot = statusSnapshot(
      selectedModel: selectedModel,
      pausedModelDownloadID: selectedModel.id
    )

    XCTAssertEqual(
      LocalModelStatusPresenter.defaultDownloadButtonTitle(snapshot),
      "Continue Granite 4.0-H-350M"
    )
  }

  func testRecoverySummaryExplainsPausedDownloadChoices() {
    let selectedModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: false,
      active: false
    )
    let summary = LocalModelOperationPresenter.recoverySummary(
      operationSnapshot(pausedModel: selectedModel, selectedModel: selectedModel)
    )

    XCTAssertTrue(summary.contains("Continue Granite 4.0-H-350M"))
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
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: false,
      active: false
    )
    let guidance = LocalModelOperationPresenter.setupGuidance(
      operationSnapshot(selectedModel: selectedModel)
    )

    XCTAssertEqual(guidance.title, "Download Local Model")
    XCTAssertTrue(guidance.detail.contains("Download size"))
    XCTAssertTrue(guidance.detail.contains("License"))
    XCTAssertFalse(guidance.title.contains("Engine"))
    XCTAssertFalse(guidance.detail.contains("|"))
  }

  func testSetupModelChoiceDetailAvoidsTechnicalSeparators() {
    let selectedModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: false,
      active: false
    )
    let detail = LocalModelOperationPresenter.setupModelChoiceDetail(
      operationSnapshot(selectedModel: selectedModel),
      defaultModelID: selectedModel.id
    )

    XCTAssertTrue(detail.contains("Granite 4.0-H-350M"))
    XCTAssertTrue(detail.contains("one verified local model"))
    XCTAssertFalse(detail.contains("|"))
    XCTAssertFalse(detail.contains("Q4_K_M"))
  }

  func testLocalModelStatusSummaryAvoidsPipeSeparators() {
    let selectedModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: true,
      active: true
    )
    let summary = LocalModelStatusPresenter.localModelStatusSummary(
      selectedModel,
      snapshot: statusSnapshot(selectedModel: selectedModel)
    )

    XCTAssertTrue(summary.contains("Ready and active"))
    XCTAssertTrue(summary.contains("License"))
    XCTAssertFalse(summary.contains("|"))
  }

  func testModelFitSummaryAvoidsInternalTags() {
    let selectedModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: false,
      active: false
    )
    let summary = LocalModelStatusPresenter.fitSummary(
      selectedModel,
      defaultModelID: selectedModel.id
    )

    XCTAssertTrue(summary.contains("Default path"))
    XCTAssertTrue(summary.contains("lightest local use"))
    XCTAssertFalse(summary.contains("tiny"))
    XCTAssertFalse(summary.contains("Q4_K_M"))
  }

  func testCatalogDescriptionsStayUserFacing() {
    for item in LocalModelCatalog.items() {
      XCTAssertFalse(item.description.contains("tools"))
      XCTAssertFalse(item.description.contains("code"))
      XCTAssertFalse(item.description.contains("long-context"))
      XCTAssertFalse(item.description.contains("Q4_K_M"))
      XCTAssertFalse(item.description.contains("memory-assisted"))
    }
  }

  func testModelMetricsSummaryAvoidsBackendTerminology() {
    let selectedModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: true,
      active: true
    )
    let snapshot = statusSnapshot(
      selectedModel: selectedModel,
      modelHealth: ModelHealthSummary(
        packID: selectedModel.id,
        displayName: selectedModel.displayName,
        backend: "llama.cpp",
        status: "ready",
        detail: "Ready",
        source: "local",
        binaryPath: "/Applications/Amentia.app/Contents/Resources/llama-cli",
        modelPath: selectedModel.installPath,
        manifestPath: "/tmp/model-pack.json",
        metrics: [
          "contextSize": "4096",
          "modelContextSize": "32768",
          "maxOutputTokens": "192",
          "backend": "llama.cpp",
        ]
      ),
      hasActiveCatalogModel: true
    )
    let summary = LocalModelStatusPresenter.metricsSummary(snapshot)

    XCTAssertEqual(
      summary,
      "Uses a compact active context for speed, with larger local context available when needed."
    )
    XCTAssertFalse(summary.contains("Backend"))
    XCTAssertFalse(summary.contains("llama"))
    XCTAssertFalse(summary.contains("/Applications"))
    XCTAssertFalse(summary.contains("tokens"))
  }

  func testFirstUseModelChoiceSummariesExplainCuratedFit() {
    let defaultModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: false,
      active: false,
      tags: ["default", "recommended", "tiny", "tools", "code"]
    )
    let recommendedModel = model(
      id: "balanced-tiny",
      displayName: "Balanced Tiny Q4_K_M",
      downloaded: false,
      active: false,
      tags: ["recommended", "tiny", "tools", "code"]
    )
    let longContextModel = model(
      id: "minicpm5-1b",
      displayName: "MiniCPM5-1B Q4_K_M",
      downloaded: false,
      active: false,
      contextSize: 8192,
      modelContextSize: 131_072,
      maxOutputTokens: 384,
      tags: ["optional", "small", "tools", "code", "long-context"]
    )

    XCTAssertTrue(
      LocalModelDisplayPresenter.setupFitSummary(
        defaultModel,
        defaultModelID: defaultModel.id
      ).contains("Fastest first setup")
    )
    XCTAssertTrue(
      LocalModelDisplayPresenter.setupFitSummary(
        recommendedModel,
        defaultModelID: defaultModel.id
      ).contains("Balanced option")
    )
    XCTAssertTrue(
      LocalModelDisplayPresenter.setupFitSummary(
        longContextModel,
        defaultModelID: defaultModel.id
      ).contains("larger project files")
    )
  }

  func testFirstUseModelChoiceSummariesShowCapabilityAndLicense() {
    let selectedModel = model(
      id: "minicpm5-1b",
      displayName: "MiniCPM5-1B Q4_K_M",
      downloaded: false,
      active: false,
      contextSize: 8192,
      modelContextSize: 131_072,
      maxOutputTokens: 384,
      tags: ["optional", "small", "long-context"]
    )

    let capability = LocalModelDisplayPresenter.setupCapabilitySummary(selectedModel)
    let footprint = LocalModelDisplayPresenter.setupFootprintSummary(selectedModel)

    XCTAssertTrue(capability.contains("larger files"))
    XCTAssertTrue(footprint.contains("Download:"))
    XCTAssertTrue(footprint.contains("License: apache-2.0"))
    XCTAssertFalse(capability.contains("Q4_K_M"))
    XCTAssertFalse(footprint.contains("Q4_K_M"))
  }

  func testSetupGuidanceDescribesThreeModelTiers() {
    let selectedModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: false,
      active: false
    )
    let guidance = LocalModelOperationPresenter.setupGuidance(
      operationSnapshot(selectedModel: selectedModel, totalModelCount: 3)
    )

    XCTAssertTrue(guidance.actionSummary.contains("fast default"))
    XCTAssertTrue(guidance.actionSummary.contains("stronger local option"))
    XCTAssertFalse(guidance.actionSummary.contains("tiny"))
    XCTAssertFalse(guidance.actionSummary.contains("small model"))
  }

  func testModelIntegrityErrorsAvoidRawPathsAndHashes() {
    let missingSize = LocalModelIntegrityError.missingSize(path: "/Users/example/model.gguf")
    let mismatch = LocalModelIntegrityError.checksumMismatch(
      displayName: "Granite 4.0-H-350M",
      expected: String(repeating: "a", count: 64),
      actual: String(repeating: "b", count: 64)
    )

    XCTAssertFalse(missingSize.localizedDescription.contains("/Users/example"))
    XCTAssertTrue(missingSize.localizedDescription.contains("downloading it again"))
    XCTAssertFalse(mismatch.localizedDescription.contains(String(repeating: "a", count: 64)))
    XCTAssertFalse(mismatch.localizedDescription.contains(String(repeating: "b", count: 64)))
    XCTAssertTrue(mismatch.localizedDescription.contains("Download it again"))
  }

  func testDownloadProgressSummaryAvoidsPipeSeparators() {
    let selectedModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
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
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: false,
      active: false
    )
    let plan = LocalModelDownloadInterruptionPlanner.cancellationPlan(model: selectedModel)

    XCTAssertEqual(plan.timelineTitle, "Model Download Cancelled")
    XCTAssertTrue(plan.runtimeDetail.contains("Granite 4.0-H-350M"))
    XCTAssertFalse(plan.runtimeDetail.contains("Q4_K_M"))
    XCTAssertFalse(plan.timelineTitle.contains("Engine"))
  }

  func testDownloadFailureCopyKeepsRawErrorOutOfUserFacingText() {
    let selectedModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: false,
      active: false
    )
    let plan = LocalModelDownloadInterruptionPlanner.plan(
      model: selectedModel,
      error: NSError(domain: "AmentiaTests", code: 1, userInfo: [
        NSLocalizedDescriptionKey: "HTTP 503 from https://example.com/model.gguf"
      ])
    )

    XCTAssertEqual(plan.timelineTitle, "Model Download Failed")
    XCTAssertTrue(plan.runtimeDetail.contains("Check your connection"))
    XCTAssertTrue(plan.timelineBody.contains("try again"))
    XCTAssertFalse(plan.runtimeDetail.contains("https://"))
    XCTAssertFalse(plan.timelineBody.contains("HTTP 503"))
    XCTAssertEqual(plan.attributes["error"], "HTTP 503 from https://example.com/model.gguf")
  }

  func testModelActivationFailureCopyAvoidsRawLocalDetails() {
    let rawError = NSError(domain: "AmentiaTests", code: 2, userInfo: [
      NSLocalizedDescriptionKey: "manifest write failed at /Users/example/model-pack.json"
    ])

    let manifestFailure = LocalModelActivationPlanner.failurePlan(
      error: LocalModelActivationPreparationError.manifestWriteFailed(rawError)
    )
    let integrityFailure = LocalModelActivationPlanner.failurePlan(
      error: LocalModelActivationPreparationError.integrityCheckFailed(rawError)
    )
    let genericFailure = LocalModelActivationPlanner.failurePlan(error: rawError)

    XCTAssertTrue(manifestFailure.runtimeDetail.contains("Restart Amentia"))
    XCTAssertFalse(manifestFailure.runtimeDetail.contains("/Users/example"))
    XCTAssertTrue(integrityFailure.runtimeDetail.contains("Download the model again"))
    XCTAssertFalse(integrityFailure.runtimeDetail.contains("manifest"))
    XCTAssertTrue(genericFailure.runtimeDetail.contains("Model selection failed"))
    XCTAssertFalse(genericFailure.runtimeDetail.contains("/Users/example"))
  }

  func testModelActivationSuccessExplainsStartupModelStart() {
    let selectedModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: true,
      active: false
    )

    let plan = LocalModelActivationPlanner.selectionPlan(
      model: selectedModel,
      manifestPath: "/tmp/model-pack.json"
    )

    XCTAssertEqual(plan.timelineTitle, "Local Model Selected")
    XCTAssertTrue(plan.timelineBody.contains("will start it before cowork begins"))
    XCTAssertTrue(plan.relaunchRunningDetail.contains("with Granite 4.0-H-350M"))
    XCTAssertTrue(plan.relaunchIdleDetail.contains("will start"))
    XCTAssertFalse(plan.timelineBody.contains("active local model"))
    XCTAssertFalse(plan.timelineBody.contains("Q4_K_M"))
  }

  func testModelSetupRefreshFailureCopyAvoidsBackendDetails() {
    let detail = LocalModelMetadataPresenter.refreshFailureDetail()

    XCTAssertTrue(detail.contains("Restart Amentia"))
    XCTAssertTrue(detail.contains("try refreshing setup again"))
    XCTAssertFalse(detail.contains("JSON-RPC"))
    XCTAssertFalse(detail.contains("manifest"))
    XCTAssertFalse(detail.contains("llama"))
  }

  func testRuntimeModelSetupFailureClassifiesIntegrityBeforeEngineFailures() {
    let checksumFailure = RuntimeModelSetupFailurePresenter.detail(
      error: NSError(domain: "AmentiaTests", code: 1, userInfo: [
        NSLocalizedDescriptionKey:
          "Local inference setup verification failed: model checksum mismatch"
      ])
    )
    let engineFailure = RuntimeModelSetupFailurePresenter.detail(
      error: NSError(domain: "AmentiaTests", code: 2, userInfo: [
        NSLocalizedDescriptionKey: "failed to launch llama.cpp backend"
      ])
    )
    let genericFailure = RuntimeModelSetupFailurePresenter.detail(
      error: NSError(domain: "AmentiaTests", code: 3, userInfo: [
        NSLocalizedDescriptionKey: "request timed out"
      ])
    )

    XCTAssertTrue(checksumFailure.contains("Re-download the model"))
    XCTAssertFalse(checksumFailure.contains("Reinstall Amentia"))
    XCTAssertTrue(engineFailure.contains("Reinstall Amentia"))
    XCTAssertTrue(genericFailure.contains("Restart Amentia"))
    XCTAssertFalse(genericFailure.contains("Re-download"))
  }

  func testSavedActiveModelInvalidationCopyAvoidsSetupInternals() {
    let missing = UserFacingFailurePresenter.savedActiveModelMissingDetail()
    let invalid = UserFacingFailurePresenter.savedActiveModelInvalidDetail()

    for detail in [missing, invalid] {
      XCTAssertTrue(detail.contains("Choose or download a model"))
      XCTAssertFalse(detail.contains("manifest"))
      XCTAssertFalse(detail.contains("setup file"))
      XCTAssertFalse(detail.contains("catalog"))
      XCTAssertFalse(detail.contains("model file"))
    }
  }

  func testRefreshGuidanceAvoidsInternalSetupInfoLanguage() {
    let guidance = LocalModelOperationPresenter.setupGuidance(operationSnapshot())

    XCTAssertEqual(guidance.title, "Refresh Model Setup")
    XCTAssertTrue(guidance.summary.contains("setup is refreshed"))
    XCTAssertFalse(guidance.detail.contains("setup info"))
  }

  func testRecoverySummaryExplainsRuntimeRelaunch() {
    let summary = LocalModelOperationPresenter.recoverySummary(
      operationSnapshot(runtimeState: .failed)
    )

    XCTAssertTrue(summary.contains("Restart Amentia"))
    XCTAssertTrue(summary.contains("selected model choices remain local"))
  }

  func testActiveWorkModelGuidanceAvoidsTurnAndStreamingLanguage() {
    let selectedModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: true,
      active: true
    )
    let snapshot = operationSnapshot(
      isLocalModelReady: true,
      hasActiveTurn: true,
      selectedModel: selectedModel,
      activeModelDisplayName: "Granite 4.0-H-350M"
    )

    let guidance = LocalModelOperationPresenter.setupGuidance(snapshot)
    let managerSummary = LocalModelOperationPresenter.managerSummary(snapshot)
    let recoverySummary = LocalModelOperationPresenter.recoverySummary(snapshot)

    XCTAssertEqual(guidance.readinessDetail, "Working")
    XCTAssertTrue(guidance.summary.contains("current work"))
    XCTAssertTrue(guidance.detail.contains("current work"))
    XCTAssertTrue(managerSummary.contains("current work"))
    XCTAssertTrue(recoverySummary.contains("current work"))
    XCTAssertFalse(guidance.summary.contains("turn"))
    XCTAssertFalse(guidance.detail.contains("turn"))
    XCTAssertFalse(guidance.readinessDetail.contains("Streaming"))
    XCTAssertFalse(managerSummary.contains("turn"))
    XCTAssertFalse(recoverySummary.contains("turn"))
  }

  func testStartupModelStartHasClearUserGuidance() {
    let selectedModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: true,
      active: true
    )
    let snapshot = operationSnapshot(
      isLocalModelReady: false,
      isCheckingModel: true,
      selectedModel: selectedModel,
      activeModelDisplayName: "Granite 4.0-H-350M"
    )

    let guidance = LocalModelOperationPresenter.setupGuidance(snapshot)

    XCTAssertEqual(guidance.title, "Starting Local Model")
    XCTAssertEqual(guidance.tone, .active)
    XCTAssertTrue(guidance.actionSummary.contains("Starting"))
    XCTAssertTrue(guidance.summary.contains("during startup"))
    XCTAssertFalse(guidance.summary.contains("probe"))
  }

  func testFailedModelCheckShowsRecoveryPath() {
    let selectedModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: true,
      active: true
    )
    let detail = "The local model did not answer."
    let guidance = LocalModelOperationPresenter.setupGuidance(operationSnapshot(
      modelCheckFailureDetail: detail,
      selectedModel: selectedModel,
      activeModelDisplayName: "Granite 4.0-H-350M"
    ))
    let status = LocalModelStatusPresenter.statusSummary(statusSnapshot(
      selectedModel: selectedModel,
      modelHealth: readyModelHealth(for: selectedModel),
      hasActiveCatalogModel: true,
      modelCheckFailureDetail: detail
    ))
    let readiness = LocalModelStatusPresenter.readinessSummary(statusSnapshot(
      selectedModel: selectedModel,
      modelHealth: readyModelHealth(for: selectedModel),
      hasActiveCatalogModel: true,
      modelCheckFailureDetail: detail
    ))

    XCTAssertEqual(guidance.title, "Model Startup Needed")
    XCTAssertEqual(guidance.readinessDetail, "Startup Failed")
    XCTAssertEqual(status, "Model startup failed")
    XCTAssertEqual(readiness, "Local model setup needs a successful start.")
    XCTAssertTrue(guidance.actionSummary.contains("Restart Amentia"))
    XCTAssertFalse(guidance.actionSummary.contains("re-download"))
    XCTAssertTrue(guidance.detail.contains("Cowork is paused"))
    XCTAssertFalse(guidance.detail.contains("re-download"))
    XCTAssertTrue(
      LocalModelStatusPresenter.detailSummary(statusSnapshot(
        selectedModel: selectedModel,
        modelHealth: readyModelHealth(for: selectedModel),
        hasActiveCatalogModel: true,
        modelCheckFailureDetail: detail
      )).contains("Cowork is paused")
    )
    XCTAssertTrue(
      LocalModelStatusPresenter.installHintSummary(statusSnapshot(
        selectedModel: selectedModel,
        modelHealth: readyModelHealth(for: selectedModel),
        hasActiveCatalogModel: true,
        modelCheckFailureDetail: detail
      )).contains("Cowork is paused")
    )
  }

  func testMissingModelEngineUsesPlainRepairCopy() {
    let selectedModel = model(
      id: "granite-4.0-h-350m",
      displayName: "Granite 4.0-H-350M Q4_K_M",
      downloaded: true,
      active: true
    )
    let health = ModelHealthSummary(
      packID: selectedModel.id,
      displayName: selectedModel.displayName,
      backend: "llama.cpp",
      status: "unavailable",
      detail: "Binary missing",
      source: "local",
      binaryPath: nil,
      modelPath: selectedModel.installPath,
      manifestPath: "/tmp/model-pack.json",
      metrics: ["readiness": "binary_missing"]
    )
    let detail = LocalModelStatusPresenter.installHintSummary(statusSnapshot(
      selectedModel: selectedModel,
      modelHealth: health
    ))

    XCTAssertTrue(detail.contains("local model engine"))
    XCTAssertFalse(detail.contains("runner"))
  }

  func testSetupPrimaryActionDoesNotOfferManualModelCheckWhenReady() {
    let action = LocalModelActionPlanner.setupPrimaryAction(actionSnapshot(
      isLocalModelReady: true
    ))

    XCTAssertNil(action)
  }

  func testFailedModelCheckDoesNotExposeManualRetryAction() {
    let action = LocalModelActionPlanner.setupPrimaryAction(actionSnapshot(
      isLocalModelReady: false
    ))

    XCTAssertNil(action)
  }

  func testModelProbeSuccessPresentationKeepsDefaultCopySimple() {
    let presentation = LocalModelProbePresenter.presentation(for: RuntimeBridge.RuntimeModelProbe(
      status: "ready",
      detail: "The active local model answered a short probe.",
      backend: "llama.cpp",
      modelID: "granite-4.0-h-350m",
      sample: "Amentia model ready."
    ))

    XCTAssertEqual(presentation.runtimeDetail, "Local model started.")
    XCTAssertEqual(presentation.timelineTitle, "Local Model Started")
    XCTAssertEqual(presentation.timelineKind, .system)
    XCTAssertEqual(presentation.attributes["sample"], "Amentia model ready.")
    XCTAssertFalse(presentation.runtimeDetail.contains("llama"))
  }

  func testModelProbeFailurePresentationGivesRecoveryPath() {
    let presentation = LocalModelProbePresenter.presentation(for: RuntimeBridge.RuntimeModelProbe(
      status: "error",
      detail: "Local llama.cpp inference failed.",
      backend: "llama.cpp",
      modelID: "granite-4.0-h-350m",
      sample: nil
    ))

    XCTAssertEqual(presentation.timelineTitle, "Local Model Startup Failed")
    XCTAssertEqual(presentation.timelineKind, .warning)
    XCTAssertTrue(presentation.runtimeDetail.contains("Cowork is paused"))
    XCTAssertTrue(presentation.runtimeDetail.contains("Restart Amentia"))
    XCTAssertFalse(presentation.runtimeDetail.contains("re-download"))
    XCTAssertFalse(presentation.timelineBody.contains("llama.cpp"))
    XCTAssertFalse(presentation.timelineBody.contains("re-download"))
    XCTAssertEqual(presentation.attributes["detail"], "Local llama.cpp inference failed.")
    XCTAssertNil(presentation.attributes["sample"])
  }

  func testModelProbeRequestFailureKeepsTimelineBodyUserFacing() {
    let presentation = LocalModelProbePresenter.requestFailurePresentation(
      error: NSError(domain: "AmentiaTests", code: 1, userInfo: [
        NSLocalizedDescriptionKey: "JSON-RPC request failed: backend pipe closed."
      ])
    )

    XCTAssertEqual(presentation.timelineTitle, "Local Model Startup Failed")
    XCTAssertTrue(presentation.runtimeDetail.contains("Cowork is paused"))
    XCTAssertTrue(presentation.runtimeDetail.contains("Restart Amentia"))
    XCTAssertFalse(presentation.timelineBody.contains("JSON-RPC"))
    XCTAssertEqual(
      presentation.attributes["detail"],
      "JSON-RPC request failed: backend pipe closed."
    )
  }

  private func statusSnapshot(
    selectedModel: LocalModelSummary,
    modelHealth: ModelHealthSummary? = nil,
    modelDownloadID: String? = nil,
    pausedModelDownloadID: String? = nil,
    progress: ModelDownloadProgress? = nil,
    hasActiveCatalogModel: Bool = false,
    modelCheckFailureDetail: String? = nil
  ) -> LocalModelStatusSnapshot {
    LocalModelStatusSnapshot(
      runtimeState: .ready,
      modelHealth: modelHealth,
      modelDownloadID: modelDownloadID,
      pausedModelDownloadID: pausedModelDownloadID,
      modelDownloadProgress: progress,
      selectedSetupModelID: selectedModel.id,
      selectedSetupModel: selectedModel,
      hasActiveCatalogModel: hasActiveCatalogModel,
      modelCheckFailureDetail: modelCheckFailureDetail
    )
  }

  private func operationSnapshot(
    runtimeState: RuntimeBridge.ConnectionState = .ready,
    isLocalModelReady: Bool = false,
    hasActiveTurn: Bool = false,
    isCheckingModel: Bool = false,
    modelCheckFailureDetail: String? = nil,
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
      isCheckingModel: isCheckingModel,
      modelCheckFailureDetail: modelCheckFailureDetail,
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

  private func actionSnapshot(
    runtimeState: RuntimeBridge.ConnectionState = .ready,
    isLocalModelReady: Bool = false,
    hasModelDownload: Bool = false,
    pausedModelDownloadID: String? = nil,
    selectedDownloadBlockedDetail: String? = nil,
    canPauseDownload: Bool = false,
    canDownloadPausedModel: Bool = false,
    canDownloadSelectedModel: Bool = false,
    canBootstrapModelPackMetadata: Bool = false,
    canCancelDownload: Bool = false,
    defaultDownloadTitle: String = "Download Model"
  ) -> LocalModelActionSnapshot {
    LocalModelActionSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady,
      hasModelDownload: hasModelDownload,
      pausedModelDownloadID: pausedModelDownloadID,
      selectedDownloadBlockedDetail: selectedDownloadBlockedDetail,
      canPauseDownload: canPauseDownload,
      canDownloadPausedModel: canDownloadPausedModel,
      canDownloadSelectedModel: canDownloadSelectedModel,
      canBootstrapModelPackMetadata: canBootstrapModelPackMetadata,
      canCancelDownload: canCancelDownload,
      defaultDownloadTitle: defaultDownloadTitle
    )
  }

  private func model(
    id: String,
    displayName: String,
    downloaded: Bool,
    active: Bool,
    contextSize: Int = 4096,
    modelContextSize: Int = 32_768,
    maxOutputTokens: Int = 192,
    tags: [String] = ["tiny"]
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
      contextSize: contextSize,
      modelContextSize: modelContextSize,
      maxOutputTokens: maxOutputTokens,
      license: "apache-2.0",
      tags: tags,
      installPath: "/tmp/\(id).gguf",
      downloaded: downloaded,
      active: active,
      localSizeBytes: downloaded ? 222_000_000 : nil
    )
  }

  private func readyModelHealth(for model: LocalModelSummary) -> ModelHealthSummary {
    ModelHealthSummary(
      packID: model.id,
      displayName: model.displayName,
      backend: "llama.cpp",
      status: "ready",
      detail: "Ready",
      source: "local",
      binaryPath: nil,
      modelPath: model.installPath,
      manifestPath: "/tmp/model-pack.json",
      metrics: [
        "contextSize": "\(model.contextSize)",
        "modelContextSize": "\(model.modelContextSize)",
        "maxOutputTokens": "\(model.maxOutputTokens)",
        "readiness": "ready",
        "packReady": "true",
      ]
    )
  }
}
