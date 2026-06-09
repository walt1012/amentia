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
}

enum LocalModelStatusPresenter {
  static func displayName(_ snapshot: LocalModelStatusSnapshot) -> String {
    snapshot.modelHealth?.displayName ?? "Local Model Not Loaded"
  }

  static func statusSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      switch snapshot.runtimeState {
      case .disconnected:
        return "Launch the local engine to inspect model setup."
      case .launching:
        return "Checking local model setup..."
      case .failed:
        return "Relaunch the local engine to recover model setup."
      case .ready:
        return "Choose and download one local model to continue."
      }
    }

    if modelHealth.status == "ready", !snapshot.hasActiveCatalogModel {
      return "Model ready outside curated catalog"
    }

    switch modelHealth.status {
    case "ready":
      return "Ready to use"
    case "unavailable":
      return "Model setup needed"
    case "error":
      return "Model needs attention"
    default:
      return "Checking model"
    }
  }

  static func showsActivity(_ snapshot: LocalModelStatusSnapshot) -> Bool {
    snapshot.runtimeState == .launching || snapshot.modelDownloadID != nil
  }

  static func detailSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      if let model = snapshot.selectedSetupModel {
        if model.downloaded {
          return "Pith will use \(model.displayName) after it is selected."
        }

        return "Pith will use \(model.displayName) after it is downloaded and selected."
      }

      return "Pith needs one downloaded local model selected before it can answer."
    }

    if modelHealth.status == "ready", !snapshot.hasActiveCatalogModel {
      return "Choose a verified model from the local catalog before running Pith. Removed or external model selections are not treated as first-use ready."
    }

    return modelHealth.detail
  }

  static func sourceSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "Model source unavailable."
    }

    return "Model source: \(modelHealth.source)"
  }

  static func metricsSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "Metrics: unavailable"
    }

    let contextSize = modelHealth.metrics["contextSize"] ?? "unknown"
    let modelContextSize = modelHealth.metrics["modelContextSize"]
      .map { "\(contextSize) active / \($0) limit" }
      ?? contextSize
    let maxOutputTokens = modelHealth.metrics["maxOutputTokens"] ?? "unknown"
    let backend = modelHealth.metrics["backend"] ?? modelHealth.backend
    return "Context: \(modelContextSize) | Max Output: \(maxOutputTokens) | Backend: \(backend)"
  }

  static func readinessSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "Model readiness unavailable."
    }

    let readiness = modelHealth.metrics["readiness"] ?? "unknown"
    let packReady = modelHealth.metrics["packReady"] ?? "false"
    if readiness == "ready", packReady == "true" {
      return "Model setup is ready."
    }
    return "Model setup is not ready yet."
  }

  static func installHintSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "Launch Pith to inspect local model setup."
    }

    return modelHealth.metrics["installHint"] ?? "Install hint unavailable."
  }

  static func suggestedPathSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard snapshot.modelHealth != nil else {
      return "Suggested model folders are unavailable until Pith finishes checking setup."
    }

    return "Use the folder buttons below if you need to inspect local model files."
  }

  static func artifactPathSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard snapshot.modelHealth != nil else {
      return "No local model files are active yet."
    }

    return "Pith is using the selected verified local model."
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

    let localSize = model.localSizeBytes.map(LocalModelByteFormatter.string)
      ?? LocalModelByteFormatter.string(model.sizeBytes)
    return "\(status) | \(localSize) | \(model.license)"
  }

  static func managerRuleSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    if snapshot.modelDownloadID != nil {
      return "Downloads can be paused or cancelled. Pith will activate only one verified model."
    }
    if snapshot.pausedModelDownloadID != nil {
      return "Continue the paused download or cancel it before starting another model."
    }
    if let model = snapshot.selectedSetupModel {
      return "Selected setup model: \(model.displayName). Pith runs one active model at a time."
    }

    return "Choose one curated local model. Pith verifies the file before it can run."
  }

  static func localModelChoiceSummary(
    _ model: LocalModelSummary,
    snapshot: LocalModelStatusSnapshot,
    defaultModelID: String
  ) -> String {
    if model.active {
      return "Currently active local model"
    }
    if snapshot.modelDownloadID == model.id {
      return "Downloading for local setup"
    }
    if snapshot.pausedModelDownloadID == model.id {
      return "Paused download"
    }
    if model.needsVerification {
      return "Found local file"
    }
    if model.id == snapshot.selectedSetupModelID {
      if model.id == defaultModelID {
        return "Selected default for first setup"
      }

      return "Selected alternative for first setup"
    }
    if model.id == defaultModelID {
      return "Default first-use choice"
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
      return "Replace"
    }

    return model.downloaded ? "Downloaded" : "Download"
  }

  static func tagSummary(_ model: LocalModelSummary) -> String {
    model.tags.joined(separator: " / ")
  }

  static func pathSummary(_ model: LocalModelSummary) -> String {
    model.installPath
  }

}
