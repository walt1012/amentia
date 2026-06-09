import Foundation

struct LocalModelActionSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let isLocalModelReady: Bool
  let hasModelDownload: Bool
  let pausedModelDownloadID: String?
  let selectedDownloadBlockedDetail: String?
  let canPauseDownload: Bool
  let canDownloadPausedModel: Bool
  let canDownloadSelectedModel: Bool
  let canBootstrapModelPackMetadata: Bool
  let canCancelDownload: Bool
  let defaultDownloadTitle: String
}

enum LocalModelPrimaryAction {
  case pauseDownload
  case continueDownload(modelID: String)
  case downloadSelectedModel
  case blockedDownload
  case bootstrapModelPackMetadata
}

enum LocalModelSecondaryAction {
  case cancelDownload
}

enum LocalModelActionPlanner {
  static func setupPrimaryAction(_ snapshot: LocalModelActionSnapshot) -> LocalModelPrimaryAction? {
    if snapshot.hasModelDownload {
      return .pauseDownload
    }
    if let pausedModelDownloadID = snapshot.pausedModelDownloadID {
      return .continueDownload(modelID: pausedModelDownloadID)
    }
    guard snapshot.runtimeState == .ready else {
      return nil
    }
    if snapshot.canDownloadSelectedModel {
      return .downloadSelectedModel
    }
    if snapshot.selectedDownloadBlockedDetail != nil {
      return .blockedDownload
    }
    if snapshot.canBootstrapModelPackMetadata {
      return .bootstrapModelPackMetadata
    }

    return nil
  }

  static func managerPrimaryAction(_ snapshot: LocalModelActionSnapshot) -> LocalModelPrimaryAction? {
    guard snapshot.runtimeState == .ready else {
      return nil
    }
    if snapshot.hasModelDownload {
      return .pauseDownload
    }
    if let pausedModelDownloadID = snapshot.pausedModelDownloadID {
      return .continueDownload(modelID: pausedModelDownloadID)
    }
    if !snapshot.isLocalModelReady {
      if snapshot.canDownloadSelectedModel {
        return .downloadSelectedModel
      }
      if snapshot.selectedDownloadBlockedDetail != nil {
        return .blockedDownload
      }
      if snapshot.canBootstrapModelPackMetadata {
        return .bootstrapModelPackMetadata
      }
    }

    return nil
  }

  static func setupSecondaryAction(_ snapshot: LocalModelActionSnapshot) -> LocalModelSecondaryAction? {
    snapshot.hasModelDownload || snapshot.pausedModelDownloadID != nil ? .cancelDownload : nil
  }

  static func managerSecondaryAction(_ snapshot: LocalModelActionSnapshot) -> LocalModelSecondaryAction? {
    snapshot.canCancelDownload ? .cancelDownload : nil
  }

  static func primaryTitle(
    for action: LocalModelPrimaryAction?,
    snapshot: LocalModelActionSnapshot
  ) -> String? {
    guard let action else {
      return nil
    }

    switch action {
    case .pauseDownload:
      return "Pause Download"
    case .continueDownload:
      return "Continue Download"
    case .downloadSelectedModel:
      return snapshot.defaultDownloadTitle
    case .blockedDownload:
      return "Download Blocked"
    case .bootstrapModelPackMetadata:
      return "Repair Engine"
    }
  }

  static func canRun(
    _ action: LocalModelPrimaryAction?,
    snapshot: LocalModelActionSnapshot
  ) -> Bool {
    guard let action else {
      return false
    }

    switch action {
    case .pauseDownload:
      return snapshot.canPauseDownload
    case .continueDownload:
      return snapshot.canDownloadPausedModel
    case .downloadSelectedModel:
      return snapshot.canDownloadSelectedModel
    case .blockedDownload:
      return false
    case .bootstrapModelPackMetadata:
      return snapshot.canBootstrapModelPackMetadata
    }
  }

  static func secondaryTitle(for action: LocalModelSecondaryAction?) -> String? {
    guard let action else {
      return nil
    }

    switch action {
    case .cancelDownload:
      return "Cancel Download"
    }
  }

  static func canRun(
    _ action: LocalModelSecondaryAction?,
    snapshot: LocalModelActionSnapshot
  ) -> Bool {
    guard let action else {
      return false
    }

    switch action {
    case .cancelDownload:
      return snapshot.canCancelDownload
    }
  }
}
