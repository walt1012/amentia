import Foundation

struct LocalModelOperationSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let isLocalModelReady: Bool
  let hasActiveTurn: Bool
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
        title: "Launch Local Engine",
        summary: "Launch Pith's local engine before choosing or running a model.",
        detail: "Model choices and downloads stay on this Mac.",
        actionSummary: "Launch Pith's local engine to continue model setup.",
        readinessDetail: "Launch",
        tone: .warning
      )
    case .launching:
      return LocalModelSetupGuidance(
        title: "Checking Local Engine",
        summary: "Pith is reconnecting your local engine and selected model.",
        detail: "Your model choices and download progress will appear after the local engine is ready.",
        actionSummary: "Checking local engine setup...",
        readinessDetail: "Checking",
        tone: .active
      )
    case .failed:
      return LocalModelSetupGuidance(
        title: "Relaunch Local Engine",
        summary: "Local engine stopped before model setup could be completed.",
        detail: "Relaunch the local engine to restore model choices, downloads, and the active model.",
        actionSummary: "Relaunch the local engine before changing models.",
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
      return "Choose one local model for Pith to download and run."
    }

    let role = model.id == defaultModelID ? "Default" : "Recommended alternative"
    let status: String
    if model.downloaded {
      status = "downloaded"
    } else if model.needsVerification {
      status = "needs verification"
    } else {
      status = "not downloaded"
    }
    return "\(role): \(model.description) Size \(LocalModelByteFormatter.string(model.sizeBytes)). License \(model.license). Status \(status). Pith runs one active model at a time."
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
      return "Downloading \(model.displayName). Pith will run one verified model at a time."
    }
    if let model = snapshot.pausedModel {
      return "Paused \(model.displayName). Continue or cancel before starting another download."
    }
    if snapshot.hasActiveTurn {
      return "Active: \(snapshot.activeModelDisplayName ?? "local engine"). Switching waits for the current turn."
    }

    let activeModel = snapshot.activeModelDisplayName ?? "none"
    let localSize = LocalModelByteFormatter.string(snapshot.downloadedLocalSizeBytes)
    return "Active: \(activeModel). Downloaded \(snapshot.downloadedModelCount) of \(snapshot.totalModelCount). Local size \(localSize)."
  }

  static func recoverySummary(_ snapshot: LocalModelOperationSnapshot) -> String {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Launch the local engine to restore model choices and paused downloads from this Mac."
    case .launching:
      return "Reconnecting local engine state before showing the next action."
    case .failed:
      return "Relaunch the local engine. Paused downloads and selected model choices remain local."
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
        summary: "\(model.displayName) is downloading. Pith will unlock local cowork after it is ready.",
        detail: modelDetail(model),
        actionSummary: "Downloading \(model.displayName). You can pause or cancel without losing control.",
        readinessDetail: "Downloading",
        tone: .active
      )
    }

    if let model = snapshot.pausedModel {
      return LocalModelSetupGuidance(
        title: "Continue Model Download",
        summary: "\(model.displayName) is paused. Continue the download or cancel to clear the partial file.",
        detail: "Partial download state is saved locally for this model.",
        actionSummary: "\(model.displayName) is paused. Continue from the saved local state or cancel to clear it.",
        readinessDetail: "Paused",
        tone: .warning
      )
    }

    if snapshot.hasActiveTurn {
      return LocalModelSetupGuidance(
        title: "Local Engine Working",
        summary: "Pith is using the active local engine for the current turn.",
        detail: "Finish or stop the current turn before switching the active model.",
        actionSummary: "Finish or stop the current turn before switching the active model.",
        readinessDetail: "Streaming",
        tone: .active
      )
    }

    if snapshot.isLocalModelReady {
      let activeModel = snapshot.activeModelDisplayName ?? "the active local model"
      return LocalModelSetupGuidance(
        title: "Local Model Ready",
        summary: "\(activeModel) is ready for local cowork.",
        detail: "Pith will use one active local model at a time for generation.",
        actionSummary: "Local model is ready for cowork.",
        readinessDetail: "Ready",
        tone: .ready
      )
    }

    if let model = snapshot.selectedSetupModel, model.downloaded {
      return LocalModelSetupGuidance(
        title: "Use Downloaded Model",
        summary: "\(model.displayName) is downloaded but not active. Use it to finish first-use setup.",
        detail: modelDetail(model),
        actionSummary: "Use the downloaded model or refresh local model setup.",
        readinessDetail: "Select",
        tone: .warning
      )
    }

    if let model = snapshot.selectedSetupModel, model.needsVerification {
      return LocalModelSetupGuidance(
        title: "Verify Local Engine",
        summary: "\(model.displayName) is already on this Mac. Verify it to finish first-use setup.",
        detail: "Pith checks the file before using it. You can replace it with a fresh download if verification fails.",
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
        summary: "\(model.displayName) cannot start downloading yet.",
        detail: blockedDetail,
        actionSummary: blockedDetail,
        readinessDetail: "Blocked",
        tone: .danger
      )
    }

    if let model = snapshot.selectedSetupModel {
      return LocalModelSetupGuidance(
        title: "Download Local Model",
        summary: "Fresh installs need one curated local model before Pith can answer locally. \(model.displayName) is selected.",
        detail: modelDetail(model),
        actionSummary: "Choose between the fastest default and the stronger tiny alternative.",
        readinessDetail: "Download",
        tone: .warning
      )
    }

    return LocalModelSetupGuidance(
      title: "Repair Model Setup",
      summary: "Local model choices are unavailable until setup is refreshed.",
      detail: "Refresh local model setup or relaunch the local engine to restore model choices.",
      actionSummary: "Refresh local model setup before choosing a model.",
      readinessDetail: snapshot.totalModelCount == 0 ? "Repair" : "Choose",
      tone: .warning
    )
  }

  private static func modelDetail(_ model: LocalModelSummary) -> String {
    let size = LocalModelByteFormatter.string(model.sizeBytes)
    let context = "\(model.contextSize) active context / \(model.modelContextSize) model limit"
    return "Size \(size). License \(model.license). Context \(context)."
  }

  private static func readyRecoverySummary(_ snapshot: LocalModelOperationSnapshot) -> String {
    if let model = snapshot.downloadingModel {
      return "Pause \(model.displayName) to keep resume data, or cancel to clear the partial file."
    }

    if let model = snapshot.pausedModel {
      return "Continue \(model.displayName) from saved resume data, or cancel to remove the partial file."
    }

    if snapshot.hasActiveTurn {
      return "Finish or stop the current turn before changing models."
    }

    if snapshot.isLocalModelReady {
      let activeModel = snapshot.activeModelDisplayName ?? "the active model"
      return "\(activeModel) is selected. Local engine relaunch will reuse this model."
    }

    if let model = snapshot.selectedSetupModel, model.downloaded {
      return "Use \(model.displayName) to activate it. Refresh local model setup if readiness still fails."
    }

    if let model = snapshot.selectedSetupModel, model.needsVerification {
      return "Verify \(model.displayName) before use, or replace it with a fresh download."
    }

    if let blockedDetail = snapshot.selectedDownloadBlockedDetail {
      return "Resolve this blocker, then retry the model download. \(blockedDetail)"
    }

    if let model = snapshot.selectedSetupModel {
      return "Download \(model.displayName). Paused downloads can continue; cancelled downloads clear partial files."
    }

    return "Refresh local model setup or relaunch the local engine to restore model choices."
  }
}
