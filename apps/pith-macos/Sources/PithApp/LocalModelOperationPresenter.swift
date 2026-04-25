import Foundation

struct LocalModelOperationSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let isLocalModelReady: Bool
  let hasActiveTurn: Bool
  let downloadingModel: LocalModelSummary?
  let pausedModel: LocalModelSummary?
  let selectedSetupModelDownloaded: Bool
  let downloadedModelCount: Int
  let totalModelCount: Int
  let activeModelDisplayName: String?
  let downloadedLocalSizeBytes: Int64
}

enum LocalModelOperationPresenter {
  static func actionSummary(_ snapshot: LocalModelOperationSnapshot) -> String {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Launch the runtime to inspect local model setup."
    case .launching:
      return "Checking local model setup..."
    case .failed:
      return "Relaunch the runtime before changing model setup."
    case .ready:
      if let model = snapshot.downloadingModel {
        return "Downloading \(model.displayName). You can pause or cancel without losing control."
      }

      if let model = snapshot.pausedModel {
        return "\(model.displayName) is paused. Continue from the saved local state or cancel to clear it."
      }

      if snapshot.hasActiveTurn {
        return "Finish or cancel the current local turn before switching the active model."
      }

      if snapshot.isLocalModelReady {
        return "Local model is ready for offline agent work."
      }

      if snapshot.downloadedModelCount == 0 {
        return "Choose a small local model to download and unlock local agent work."
      }

      if snapshot.selectedSetupModelDownloaded {
        return "Use the selected downloaded model or reinstall pack metadata to repair readiness."
      }

      return "Select a downloaded model or download the currently selected local baseline."
    }
  }

  static func isActionBlocking(_ snapshot: LocalModelOperationSnapshot) -> Bool {
    snapshot.runtimeState == .failed
      || snapshot.hasActiveTurn
      || (snapshot.runtimeState == .ready
        && !snapshot.isLocalModelReady
        && snapshot.downloadingModel == nil)
  }

  static func managerSummary(_ snapshot: LocalModelOperationSnapshot) -> String {
    let activeModel = snapshot.activeModelDisplayName ?? "none"
    let downloadSummary = snapshot.downloadingModel.map { " | Downloading: \($0.displayName)" } ?? ""
    let pausedSummary = snapshot.pausedModel.map { " | Paused: \($0.displayName)" } ?? ""
    let switchSummary = snapshot.hasActiveTurn ? " | Switch: waiting for current turn" : ""
    let localSize = formattedByteCount(snapshot.downloadedLocalSizeBytes)
    return "Downloaded: \(snapshot.downloadedModelCount)/\(snapshot.totalModelCount) | Local Size: \(localSize) | Active: \(activeModel)\(downloadSummary)\(pausedSummary)\(switchSummary)"
  }

  private static func formattedByteCount(_ byteCount: Int64) -> String {
    let formatter = ByteCountFormatter()
    formatter.countStyle = .file
    return formatter.string(fromByteCount: byteCount)
  }
}
