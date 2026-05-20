import Foundation

struct ComposerStatusSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let modelSetupTitle: String
  let modelSetupSummary: String
  let isLocalModelReady: Bool
  let hasWorkspace: Bool
  let hasRuntimeThreadSelection: Bool
  let hasActiveTurn: Bool
  let isWaitingForFirstMessage: Bool
  let hasDraftMessage: Bool
}

enum ComposerStatusPresenter {
  static func placeholder(_ snapshot: ComposerStatusSnapshot) -> String {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Launch the local runtime to start"
    case .launching:
      return "Runtime is starting..."
    case .failed:
      return "Relaunch the runtime to recover"
    case .ready:
      break
    }

    if !snapshot.isLocalModelReady {
      return snapshot.modelSetupTitle
    }

    if !snapshot.hasWorkspace {
      return "Open a workspace to start local work"
    }

    if !snapshot.hasRuntimeThreadSelection {
      return "Create or select a thread"
    }

    if snapshot.hasActiveTurn {
      return "Pith is running a local execution. Cancel to stop it."
    }

    if snapshot.isWaitingForFirstMessage {
      return snapshot.hasDraftMessage
        ? "Review the first local request, then send"
        : "Choose a starter prompt or type the first local request"
    }

    return "Ask Pith to inspect files, review diffs, or make a safe local change"
  }

  static func statusSummary(_ snapshot: ComposerStatusSnapshot) -> String {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Launch the local runtime to start."
    case .launching:
      return "Launching the local runtime..."
    case .failed:
      return "Relaunch the local runtime to recover."
    case .ready:
      if !snapshot.isLocalModelReady {
        return "\(snapshot.modelSetupSummary) Continue model setup to run locally."
      }

      if !snapshot.hasWorkspace {
        return "Open a workspace to bind tools to a local project."
      }

      if !snapshot.hasRuntimeThreadSelection {
        return "Create a thread to start local work."
      }

      if snapshot.hasActiveTurn {
        return "Pith is running locally. Cancel the execution if it is no longer useful."
      }

      if snapshot.isWaitingForFirstMessage {
        if snapshot.hasDraftMessage {
          return "Review the starter prompt, then send the first local request."
        }
        return "Choose a starter prompt or type a short local request."
      }

      return "Ready for local work."
    }
  }
}
