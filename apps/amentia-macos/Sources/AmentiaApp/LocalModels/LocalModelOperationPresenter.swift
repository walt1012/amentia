import Foundation

struct LocalModelOperationSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let isLocalModelReady: Bool
  let hasActiveTurn: Bool
  let isCheckingModel: Bool
  let hasPendingModelCheck: Bool
  let modelCheckFailureDetail: String?
  let downloadingModel: LocalModelSummary?
  let pausedModel: LocalModelSummary?
  let selectedSetupModel: LocalModelSummary?
  let selectedDownloadBlockedDetail: String?
  let downloadedModelCount: Int
  let totalModelCount: Int
  let activeModelDisplayName: String?
  let downloadedLocalSizeBytes: Int64
}

struct LocalModelSetupGuidance {
  let title: String
  let summary: String
  let detail: String
  let actionSummary: String
  let readinessDetail: String
  let tone: StatusTone
}

enum LocalModelOperationPresenter {
  static func setupGuidance(_ snapshot: LocalModelOperationSnapshot) -> LocalModelSetupGuidance {
    switch snapshot.runtimeState {
    case .disconnected:
      return LocalModelSetupGuidance(
        title: "Start Amentia",
        summary: "Start Amentia before choosing or running a model.",
        detail: "Model choices and downloads stay on this Mac.",
        actionSummary: "Start Amentia to continue model setup.",
        readinessDetail: "Launch",
        tone: .warning
      )
    case .launching:
      return LocalModelSetupGuidance(
        title: "Starting Amentia",
        summary: "Amentia is reconnecting your selected local model.",
        detail: "Your model choices and download progress will appear after Amentia is ready.",
        actionSummary: "Checking Amentia setup...",
        readinessDetail: "Checking",
        tone: .active
      )
    case .failed:
      return LocalModelSetupGuidance(
        title: "Restart Amentia",
        summary: "Amentia stopped before model setup could be completed.",
        detail: "Restart Amentia to restore model choices, downloads, and the active model.",
        actionSummary: "Restart Amentia before changing models.",
        readinessDetail: "Relaunch",
        tone: .danger
      )
    case .ready:
      return readySetupGuidance(snapshot)
    }
  }

  static func actionSummary(_ snapshot: LocalModelOperationSnapshot) -> String {
    setupGuidance(snapshot).actionSummary
  }

  static func setupModelChoiceDetail(
    _ snapshot: LocalModelOperationSnapshot,
    defaultModelID: String
  ) -> String {
    guard let model = snapshot.selectedSetupModel else {
      return "Choose one local model for Amentia to download and run."
    }

    let role = model.id == defaultModelID ? "Default choice" : "Alternative"
    let status: String
    if model.downloaded {
      status = "downloaded"
    } else if model.needsVerification {
      status = "needs verification"
    } else {
      status = "not downloaded"
    }
    let modelName = LocalModelDisplayPresenter.actionName(model)
    let size = LocalModelByteFormatter.string(model.sizeBytes)
    return "\(role): \(modelName) is \(status). About \(size). Amentia runs one verified local model at a time."
  }

  static func isActionBlocking(_ snapshot: LocalModelOperationSnapshot) -> Bool {
    snapshot.runtimeState == .failed
      || snapshot.hasActiveTurn
      || (snapshot.runtimeState == .ready
        && !snapshot.isLocalModelReady
        && snapshot.downloadingModel == nil)
  }

  static func managerSummary(_ snapshot: LocalModelOperationSnapshot) -> String {
    if let model = snapshot.downloadingModel {
      return "Downloading \(displayName(model)). Amentia will run one verified model at a time."
    }
    if let model = snapshot.pausedModel {
      return "Paused \(displayName(model)). Continue or cancel before starting another download."
    }
    if snapshot.hasActiveTurn {
      return "Active: \(snapshot.activeModelDisplayName ?? "local model"). Switching waits for the current work."
    }
    if snapshot.isCheckingModel {
      return "Checking the active local model before using it for cowork."
    }
    if snapshot.hasPendingModelCheck {
      return "Selected: \(snapshot.activeModelDisplayName ?? "local model"). Amentia will confirm it can answer after startup settles."
    }
    if snapshot.modelCheckFailureDetail != nil {
      return "The active local model needs a successful check before cowork can continue."
    }

    let activeModel = snapshot.activeModelDisplayName ?? "none"
    let localSize = LocalModelByteFormatter.string(snapshot.downloadedLocalSizeBytes)
    return "Active: \(activeModel). \(snapshot.downloadedModelCount) of \(snapshot.totalModelCount) models are ready. They use \(localSize) on this Mac."
  }

  static func recoverySummary(_ snapshot: LocalModelOperationSnapshot) -> String {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Start Amentia to restore model choices and paused downloads from this Mac."
    case .launching:
      return "Reconnecting Amentia before showing the next action."
    case .failed:
      return "Restart Amentia. Paused downloads and selected model choices remain local."
    case .ready:
      return readyRecoverySummary(snapshot)
    }
  }

  private static func readySetupGuidance(
    _ snapshot: LocalModelOperationSnapshot
  ) -> LocalModelSetupGuidance {
    if let model = snapshot.downloadingModel {
      return LocalModelSetupGuidance(
        title: "Downloading Local Model",
        summary: "\(displayName(model)) is downloading. Amentia will unlock local cowork after it is ready.",
        detail: modelDetail(model),
        actionSummary: "Downloading \(displayName(model)). You can pause or cancel without losing control.",
        readinessDetail: "Downloading",
        tone: .active
      )
    }

    if let model = snapshot.pausedModel {
      return LocalModelSetupGuidance(
        title: "Continue Model Download",
        summary: "\(displayName(model)) is paused. Continue the download or cancel to clear the partial file.",
        detail: "Partial download state is saved locally for this model.",
        actionSummary: "\(displayName(model)) is paused. Continue from the saved local state or cancel to clear it.",
        readinessDetail: "Paused",
        tone: .warning
      )
    }

    if snapshot.hasActiveTurn {
      return LocalModelSetupGuidance(
        title: "Local Model Working",
        summary: "Amentia is using the active local model for current work.",
        detail: "Finish or stop the current work before switching the active model.",
        actionSummary: "Finish or stop the current work before switching the active model.",
        readinessDetail: "Working",
        tone: .active
      )
    }

    if snapshot.isCheckingModel {
      return LocalModelSetupGuidance(
        title: "Checking Local Model",
        summary: "Amentia is running the final local check before cowork unlocks.",
        detail: "This short check runs on this Mac and confirms the selected model can answer.",
        actionSummary: "Checking the selected model before cowork starts...",
        readinessDetail: "Checking",
        tone: .active
      )
    }

    if snapshot.hasPendingModelCheck {
      let activeModel = snapshot.activeModelDisplayName ?? "the selected local model"
      return LocalModelSetupGuidance(
        title: "Almost Ready",
        summary: "\(activeModel) is selected. Amentia will run one final local check next.",
        detail: "After this check passes, you can start cowork prompts.",
        actionSummary: "Waiting for the final local model check.",
        readinessDetail: "Checking",
        tone: .active
      )
    }

    if let failureDetail = snapshot.modelCheckFailureDetail {
      return LocalModelSetupGuidance(
        title: "Model Check Needed",
        summary: "The active local model did not answer successfully.",
        detail: modelCheckRecoveryDetail(failureDetail),
        actionSummary: "Check the model again, restart Amentia, or re-download the selected model.",
        readinessDetail: "Check Failed",
        tone: .warning
      )
    }

    if snapshot.isLocalModelReady {
      let activeModel = snapshot.activeModelDisplayName ?? "the active local model"
      return LocalModelSetupGuidance(
        title: "Local Model Ready",
        summary: "\(activeModel) is ready for local cowork.",
        detail: "You can start cowork prompts now. Amentia will use one active local model at a time.",
        actionSummary: "Local model is ready for cowork.",
        readinessDetail: "Ready",
        tone: .ready
      )
    }

    if let model = snapshot.selectedSetupModel, model.downloaded {
      return LocalModelSetupGuidance(
        title: "Use Downloaded Model",
        summary: "\(displayName(model)) is downloaded but not active. Use it to finish first-use setup.",
        detail: modelDetail(model),
        actionSummary: "Use the downloaded model or refresh local model setup.",
        readinessDetail: "Select",
        tone: .warning
      )
    }

    if let model = snapshot.selectedSetupModel, model.needsVerification {
      return LocalModelSetupGuidance(
        title: "Verify Local Model",
        summary: "\(displayName(model)) is already on this Mac. Verify it to finish first-use setup.",
        detail: "Amentia checks the file before using it. You can replace it with a fresh download if verification fails.",
        actionSummary: "Verify the selected local model or replace it with a fresh download.",
        readinessDetail: "Verify",
        tone: .warning
      )
    }

    if let model = snapshot.selectedSetupModel,
       let blockedDetail = snapshot.selectedDownloadBlockedDetail
    {
      return LocalModelSetupGuidance(
        title: "Download Blocked",
        summary: "\(displayName(model)) cannot start downloading yet.",
        detail: blockedDetail,
        actionSummary: blockedDetail,
        readinessDetail: "Blocked",
        tone: .danger
      )
    }

    if let model = snapshot.selectedSetupModel {
      return LocalModelSetupGuidance(
        title: "Download Local Model",
        summary: "Fresh installs need one curated local model before Amentia can answer locally. \(LocalModelDisplayPresenter.actionName(model)) is selected.",
        detail: modelDetail(model),
        actionSummary: "Choose the fast default or a stronger local option.",
        readinessDetail: "Download",
        tone: .warning
      )
    }

    return LocalModelSetupGuidance(
      title: "Refresh Model Setup",
      summary: "Local model choices are unavailable until setup is refreshed.",
      detail: "Refresh local model setup or restart Amentia to restore model choices.",
      actionSummary: "Refresh local model setup before choosing a model.",
      readinessDetail: snapshot.totalModelCount == 0 ? "Repair" : "Choose",
      tone: .warning
    )
  }

  private static func modelDetail(_ model: LocalModelSummary) -> String {
    let size = LocalModelByteFormatter.string(model.sizeBytes)
    return "Download size: \(size). License: \(model.license). Amentia verifies the file before it can run."
  }

  private static func readyRecoverySummary(_ snapshot: LocalModelOperationSnapshot) -> String {
    if let model = snapshot.downloadingModel {
      return "Pause \(displayName(model)) to keep resume data, or cancel to clear the partial file."
    }

    if let model = snapshot.pausedModel {
      return "Continue \(displayName(model)) from saved resume data, or cancel to remove the partial file."
    }

    if snapshot.hasActiveTurn {
      return "Finish or stop the current work before changing models."
    }

    if snapshot.isCheckingModel {
      return "Amentia is running a short local check to confirm the active model can answer."
    }

    if snapshot.hasPendingModelCheck {
      return "Amentia will check the active model as soon as startup and setup are ready."
    }

    if let failureDetail = snapshot.modelCheckFailureDetail {
      return modelCheckRecoveryDetail(failureDetail)
    }

    if snapshot.isLocalModelReady {
      let activeModel = snapshot.activeModelDisplayName ?? "the active model"
      return "\(activeModel) is selected. Restarting Amentia will reuse this model."
    }

    if let model = snapshot.selectedSetupModel, model.downloaded {
      return "Use \(displayName(model)) to activate it. Refresh local model setup if readiness still fails."
    }

    if let model = snapshot.selectedSetupModel, model.needsVerification {
      return "Verify \(displayName(model)) before use, or replace it with a fresh download."
    }

    if let blockedDetail = snapshot.selectedDownloadBlockedDetail {
      return "Resolve this blocker, then retry the model download. \(blockedDetail)"
    }

    if let model = snapshot.selectedSetupModel {
      return "Download \(displayName(model)). Paused downloads can continue; cancelled downloads clear partial files."
    }

    return "Refresh local model setup or restart Amentia to restore model choices."
  }

  private static func displayName(_ model: LocalModelSummary) -> String {
    LocalModelDisplayPresenter.actionName(model)
  }

  private static func modelCheckRecoveryDetail(_ failureDetail: String) -> String {
    let detail = failureDetail.trimmingCharacters(in: .whitespacesAndNewlines)
    let recoveryDetail = LocalModelProbePresenter.readinessFailureDetail()

    if detail.isEmpty {
      return recoveryDetail
    }
    if detail.contains("Cowork is paused") {
      return detail
    }
    return "\(detail) \(recoveryDetail)"
  }
}
