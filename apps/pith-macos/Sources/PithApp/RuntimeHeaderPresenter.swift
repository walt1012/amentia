import Foundation

struct RuntimeHeaderSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let runtimeDetail: String
  let modelSetupSummary: String
  let isLocalModelReady: Bool
  let hasWorkspace: Bool
  let hasRuntimeThreadSelection: Bool
  let hasActiveTurn: Bool
  let isWaitingForFirstMessage: Bool
  let hasDraftMessage: Bool
  let isWorkspaceSearching: Bool
  let hasModelDownload: Bool
  let hasPausedModelDownload: Bool
}

enum RuntimeHeaderPresenter {
  static func statusSummary(_ snapshot: RuntimeHeaderSnapshot) -> String {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Launch the local runtime to restore model, workspace, plugins, and memory."
    case .launching:
      return "Starting the local runtime and reconnecting app state..."
    case .failed:
      return "Runtime stopped. Relaunch to recover the local agent loop."
    case .ready:
      if !snapshot.isLocalModelReady {
        return snapshot.modelSetupSummary
      }
      if !snapshot.hasWorkspace {
        return "Model is ready. Open a workspace to bind tools to a project."
      }
      if snapshot.hasActiveTurn {
        return "Pith is streaming locally. Cancel only if the turn is no longer useful."
      }
      if !snapshot.hasRuntimeThreadSelection {
        return "Select or create a thread to start local agent work."
      }
      if snapshot.isWaitingForFirstMessage {
        return snapshot.hasDraftMessage
          ? "First local request is drafted. Send it to finish setup."
          : "Ready for the first local request."
      }
      return "Ready for local agent work."
    }
  }

  static func statusTone(_ snapshot: RuntimeHeaderSnapshot) -> StatusTone {
    switch snapshot.runtimeState {
    case .disconnected:
      return .warning
    case .launching:
      return .active
    case .failed:
      return .danger
    case .ready:
      if snapshot.hasActiveTurn || snapshot.hasModelDownload || snapshot.isWorkspaceSearching {
        return .active
      }
      if !snapshot.hasWorkspace
        || !snapshot.isLocalModelReady
        || !snapshot.hasRuntimeThreadSelection
        || snapshot.isWaitingForFirstMessage
      {
        return .warning
      }
      return .ready
    }
  }

  static func showsActivity(_ snapshot: RuntimeHeaderSnapshot) -> Bool {
    snapshot.runtimeState == .launching
      || snapshot.isWorkspaceSearching
      || snapshot.hasModelDownload
      || snapshot.hasActiveTurn
  }

  static func shouldShowDetail(_ snapshot: RuntimeHeaderSnapshot) -> Bool {
    guard !snapshot.runtimeDetail.isEmpty else {
      return false
    }

    switch snapshot.runtimeState {
    case .disconnected:
      return snapshot.runtimeDetail != "Runtime not launched"
    case .launching, .failed:
      return true
    case .ready:
      return snapshot.hasActiveTurn
        || snapshot.hasModelDownload
        || snapshot.hasPausedModelDownload
        || snapshot.isWorkspaceSearching
        || !snapshot.isLocalModelReady
        || !snapshot.hasWorkspace
        || !snapshot.hasRuntimeThreadSelection
        || snapshot.isWaitingForFirstMessage
    }
  }
}
