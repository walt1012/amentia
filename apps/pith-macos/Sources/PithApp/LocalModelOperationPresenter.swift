import Foundation

struct LocalModelOperationSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let isLocalModelReady: Bool
  let hasActiveTurn: Bool
  let downloadingModel: LocalModelSummary?
  let pausedModel: LocalModelSummary?
  let selectedSetupModel: LocalModelSummary?
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
        title: "Launch Local Runtime",
        summary: "Launch Pith's local runtime before choosing or running a model.",
        detail: "The model catalog, downloads, and active model state stay inside the local runtime.",
        actionSummary: "Launch the runtime to inspect local model setup.",
        readinessDetail: "Launch",
        tone: .warning
      )
    case .launching:
      return LocalModelSetupGuidance(
        title: "Checking Local Model",
        summary: "Pith is reconnecting the local model catalog and active model state.",
        detail: "Model choices and download state will appear after the runtime is ready.",
        actionSummary: "Checking local model setup...",
        readinessDetail: "Checking",
        tone: .active
      )
    case .failed:
      return LocalModelSetupGuidance(
        title: "Relaunch Runtime",
        summary: "Runtime stopped before local model setup could be completed.",
        detail: "Relaunch the runtime to recover model catalog, download, and activation state.",
        actionSummary: "Relaunch the runtime before changing model setup.",
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
      return "Choose one local model to download and run."
    }

    let role = model.id == defaultModelID ? "Default" : "Recommended alternative"
    let status = model.downloaded ? "downloaded" : "not downloaded"
    return "\(role): \(model.description) \(formattedByteCount(model.sizeBytes)) | \(model.license) | \(status). The first-use catalog is intentionally small, and Pith runs one active model at a time."
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
      return "Downloading \(model.displayName). One active model will run at a time."
    }
    if let model = snapshot.pausedModel {
      return "Paused \(model.displayName). Continue or cancel before starting another download."
    }
    if snapshot.hasActiveTurn {
      return "Active: \(snapshot.activeModelDisplayName ?? "local model"). Switching waits for the current turn."
    }

    let activeModel = snapshot.activeModelDisplayName ?? "none"
    let localSize = formattedByteCount(snapshot.downloadedLocalSizeBytes)
    return "Active: \(activeModel) | \(snapshot.downloadedModelCount)/\(snapshot.totalModelCount) downloaded | \(localSize)"
  }

  private static func formattedByteCount(_ byteCount: Int64) -> String {
    let formatter = ByteCountFormatter()
    formatter.countStyle = .file
    return formatter.string(fromByteCount: byteCount)
  }

  private static func readySetupGuidance(
    _ snapshot: LocalModelOperationSnapshot
  ) -> LocalModelSetupGuidance {
    if let model = snapshot.downloadingModel {
      return LocalModelSetupGuidance(
        title: "Downloading Local Model",
        summary: "\(model.displayName) is downloading. Pith will unlock offline agent work after it is ready.",
        detail: modelDetail(model),
        actionSummary: "Downloading \(model.displayName). You can pause or cancel without losing control.",
        readinessDetail: "Downloading",
        tone: .active
      )
    }

    if let model = snapshot.pausedModel {
      return LocalModelSetupGuidance(
        title: "Continue Local Model Download",
        summary: "\(model.displayName) is paused. Continue the download or cancel to clear the partial file.",
        detail: "Partial download state is saved locally for this model.",
        actionSummary: "\(model.displayName) is paused. Continue from the saved local state or cancel to clear it.",
        readinessDetail: "Paused",
        tone: .warning
      )
    }

    if snapshot.hasActiveTurn {
      return LocalModelSetupGuidance(
        title: "Local Model Running",
        summary: "Pith is using the active local model for the current turn.",
        detail: "Finish or cancel the current local turn before switching the active model.",
        actionSummary: "Finish or cancel the current local turn before switching the active model.",
        readinessDetail: "Streaming",
        tone: .active
      )
    }

    if snapshot.isLocalModelReady {
      let activeModel = snapshot.activeModelDisplayName ?? "the active local model"
      return LocalModelSetupGuidance(
        title: "Local Model Ready",
        summary: "\(activeModel) is ready for offline agent work.",
        detail: "Pith will use one active local model at a time for generation.",
        actionSummary: "Local model is ready for offline agent work.",
        readinessDetail: "Ready",
        tone: .ready
      )
    }

    if let model = snapshot.selectedSetupModel, model.downloaded {
      return LocalModelSetupGuidance(
        title: "Select Downloaded Local Model",
        summary: "\(model.displayName) is downloaded but not active. Use it to finish first-use setup.",
        detail: modelDetail(model),
        actionSummary: "Use the selected downloaded model or reinstall pack metadata to repair readiness.",
        readinessDetail: "Select",
        tone: .warning
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
      title: "Install Model Metadata",
      summary: "Local model choices are unavailable until model metadata is installed.",
      detail: "Install metadata or relaunch the runtime to refresh model catalog state.",
      actionSummary: "Install local model metadata before choosing a model.",
      readinessDetail: snapshot.totalModelCount == 0 ? "Metadata" : "Choose",
      tone: .warning
    )
  }

  private static func modelDetail(_ model: LocalModelSummary) -> String {
    "\(formattedByteCount(model.sizeBytes)) | \(model.license) | \(model.contextSize) context"
  }
}
