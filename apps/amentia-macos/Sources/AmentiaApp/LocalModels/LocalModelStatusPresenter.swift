import Foundation

struct LocalModelStatusSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let modelHealth: ModelHealthSummary?
  let modelDownloadID: String?
  let pausedModelDownloadID: String?
  let modelDownloadProgress: ModelDownloadProgress?
  let selectedSetupModelID: String
  let selectedSetupModel: LocalModelSummary?
  let hasActiveCatalogModel: Bool
  let modelCheckFailureDetail: String?
}

enum LocalModelStatusPresenter {
  static func displayName(_ snapshot: LocalModelStatusSnapshot) -> String {
    snapshot.modelHealth
      .map { LocalModelDisplayPresenter.cleanDisplayName($0.displayName) }
      ?? "Model Setup Not Ready"
  }

  static func statusSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      switch snapshot.runtimeState {
      case .disconnected:
        return "Start Amentia to inspect model setup."
      case .launching:
        return "Starting Amentia setup..."
      case .failed:
        return "Restart Amentia to recover model setup."
      case .ready:
        return "Choose and download one local model to continue."
      }
    }

    if modelHealth.status == "ready", !snapshot.hasActiveCatalogModel {
      return "Choose a verified model"
    }
    if snapshot.modelCheckFailureDetail != nil {
      return "Model startup failed"
    }

    switch modelHealth.status {
    case "ready":
      return "Ready to use"
    case "unavailable":
      return "Local model setup needed"
    case "error":
      return "Model needs attention"
    default:
      return "Starting local model"
    }
  }

  static func showsActivity(_ snapshot: LocalModelStatusSnapshot) -> Bool {
    snapshot.runtimeState == .launching || snapshot.modelDownloadID != nil
  }

  static func detailSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      if let model = snapshot.selectedSetupModel {
        if model.downloaded {
          return "Amentia will use \(LocalModelDisplayPresenter.actionName(model)) after it is selected."
        }

        return "Amentia will use \(LocalModelDisplayPresenter.actionName(model)) after it is downloaded and selected."
      }

      return "Amentia needs one downloaded local model selected before it can answer."
    }

    if modelHealth.status == "ready", !snapshot.hasActiveCatalogModel {
      return "Choose a verified model from Amentia's curated list before running. "
        + "Removed or external model selections need to be picked again."
    }
    if let failureDetail = snapshot.modelCheckFailureDetail {
      return LocalModelProbePresenter.recoveryDetail(for: failureDetail)
    }

    if modelHealth.status != "ready" {
      return userFacingModelRepairDetail(snapshot, modelHealth: modelHealth)
    }

    return "The selected local model is ready. Amentia will use it for cowork tasks on this Mac."
  }

  static func sourceSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "No local model is selected yet."
    }

    if modelHealth.source == "local" || modelHealth.source == "default-manifest" {
      return "Installed locally on this Mac."
    }

    return "Model setup was restored from Amentia local data."
  }

  static func metricsSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "Model details will appear after setup."
    }

    let contextSize = Int(modelHealth.metrics["contextSize"] ?? "")
    let modelContextSize = Int(modelHealth.metrics["modelContextSize"] ?? "")
    if let contextSize,
       let modelContextSize,
       modelContextSize > contextSize {
      return "Uses a compact active context for speed, with larger local context available when needed."
    }

    return "Tuned for concise local responses on this Mac."
  }

  static func readinessSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "Model setup unavailable."
    }

    let readiness = modelHealth.metrics["readiness"] ?? "unknown"
    let packReady = modelHealth.metrics["packReady"] ?? "false"
    if readiness == "ready", packReady == "true" {
      if snapshot.modelCheckFailureDetail != nil {
        return "Local model setup needs a successful start."
      }

      return "Local model setup is ready."
    }
    return "Local model setup is not ready yet."
  }

  static func installHintSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "Launch Amentia to inspect local model setup."
    }

    if modelHealth.status == "ready" {
      if let failureDetail = snapshot.modelCheckFailureDetail {
        return LocalModelProbePresenter.recoveryDetail(for: failureDetail)
      }

      return "Local model setup is ready."
    }

    return userFacingModelRepairDetail(snapshot, modelHealth: modelHealth)
  }

  private static func userFacingModelRepairDetail(
    _ snapshot: LocalModelStatusSnapshot,
    modelHealth: ModelHealthSummary
  ) -> String {
    let selectedModelName = snapshot.selectedSetupModel
      .map(LocalModelDisplayPresenter.actionName)
      ?? LocalModelDisplayPresenter.cleanDisplayName(modelHealth.displayName)
    let readiness = modelHealth.metrics["readiness"] ?? "unknown"

    switch readiness {
    case "model_missing", "manifest_only", "unconfigured":
      return "Download \(selectedModelName) in Amentia, then Amentia will select and run it automatically."
    case "binary_missing":
      return "Amentia's local model engine is missing from the app package. Reinstall Amentia from the latest release."
    case "misconfigured":
      return "Model setup is incomplete. Refresh model setup, or re-download the selected model."
    default:
      return "Model setup needs attention. Refresh setup or download a verified local model."
    }
  }

  static func suggestedPathSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard snapshot.modelHealth != nil else {
      return "Local folders appear after Amentia finishes startup."
    }

    return "Advanced: inspect downloaded model files only if setup keeps failing."
  }

  static func artifactPathSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard snapshot.modelHealth != nil else {
      return "No local model is active yet."
    }

    return "Amentia manages the selected verified model locally."
  }

  static func localModelStatusSummary(
    _ model: LocalModelSummary,
    snapshot: LocalModelStatusSnapshot
  ) -> String {
    let status: String
    if snapshot.modelDownloadID == model.id {
      status = "downloading"
    } else if snapshot.pausedModelDownloadID == model.id {
      status = "paused"
    } else if model.active {
      status = "active"
    } else if model.downloaded {
      status = "downloaded"
    } else if model.needsVerification {
      status = "verify before use"
    } else {
      status = "available"
    }

    return LocalModelDisplayPresenter.statusMetadata(
      status: status,
      sizeBytes: model.localSizeBytes ?? model.sizeBytes,
      license: model.license
    )
  }

  static func managerRuleSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    if snapshot.modelDownloadID != nil {
      return "Downloads can be paused or cancelled. Amentia activates only one verified model."
    }
    if snapshot.pausedModelDownloadID != nil {
      return "Continue the paused download or cancel it before starting another model."
    }
    if let model = snapshot.selectedSetupModel {
      return "Selected: \(LocalModelDisplayPresenter.actionName(model)). Amentia runs one active model at a time."
    }

    return "Choose one curated local model. Amentia verifies the file before it can run."
  }

  static func localModelChoiceSummary(
    _ model: LocalModelSummary,
    snapshot: LocalModelStatusSnapshot,
    defaultModelID: String
  ) -> String {
    if model.active {
      return "Active now"
    }
    if snapshot.modelDownloadID == model.id {
      return "Downloading"
    }
    if snapshot.pausedModelDownloadID == model.id {
      return "Paused download"
    }
    if model.needsVerification {
      return "Found local file"
    }
    if model.id == snapshot.selectedSetupModelID {
      if model.id == defaultModelID {
        return "Selected default"
      }

      return "Selected alternative"
    }
    if model.id == defaultModelID {
      return "Default choice"
    }
    if model.tags.contains("recommended") {
      return "Curated stronger tiny alternative"
    }

    return "Optional curated local model"
  }

  static func defaultDownloadButtonTitle(_ snapshot: LocalModelStatusSnapshot) -> String {
    let setupModelID = snapshot.selectedSetupModel?.id ?? snapshot.selectedSetupModelID
    let modelName = snapshot.selectedSetupModel.map(LocalModelDisplayPresenter.actionName)
    if snapshot.modelDownloadID == setupModelID {
      return modelName.map { "Downloading \($0)" } ?? "Downloading Model"
    }
    if snapshot.pausedModelDownloadID == setupModelID {
      return modelName.map { "Continue \($0)" } ?? "Continue Model"
    }
    if let setupModel = snapshot.selectedSetupModel {
      if setupModel.active {
        return "Model Selected"
      }
      if setupModel.needsVerification {
        return "Verify \(LocalModelDisplayPresenter.actionName(setupModel))"
      }
      if setupModel.downloaded {
        return "Use \(LocalModelDisplayPresenter.actionName(setupModel))"
      }

      return "Download \(LocalModelDisplayPresenter.actionName(setupModel))"
    }

    return "Download Selected"
  }

  static func downloadButtonTitle(
    _ model: LocalModelSummary,
    snapshot: LocalModelStatusSnapshot
  ) -> String {
    if snapshot.modelDownloadID == model.id {
      return "Downloading"
    }
    if snapshot.pausedModelDownloadID == model.id {
      return "Continue"
    }
    if model.needsVerification {
      return "Re-download"
    }

    return model.downloaded ? "Downloaded" : "Download"
  }

  static func fitSummary(_ model: LocalModelSummary, defaultModelID: String) -> String {
    LocalModelDisplayPresenter.firstUseFit(model, defaultModelID: defaultModelID)
  }

  static func pathSummary(_ model: LocalModelSummary) -> String {
    model.installPath
  }

}
