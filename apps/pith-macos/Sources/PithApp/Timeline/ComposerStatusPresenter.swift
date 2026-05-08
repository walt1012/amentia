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
      return "Open a workspace to start local agent work"
    }

    if !snapshot.hasRuntimeThreadSelection {
      return "Create or select a thread"
    }

    if snapshot.hasActiveTurn {
      return "Pith is streaming a response. Cancel to stop the current turn."
    }

    if snapshot.isWaitingForFirstMessage {
      return snapshot.hasDraftMessage
        ? "Review the first local request, then send"
        : "Choose a starter prompt or type the first local request"
    }

    return "Ask Pith to inspect files, review diffs, run shell commands, or write files"
  }

  static func statusSummary(_ snapshot: ComposerStatusSnapshot) -> String {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Use the Runtime chip or Command-R to launch the local agent loop."
    case .launching:
      return "Launching the local runtime..."
    case .failed:
      return "Use the Runtime chip or Command-R to recover the local agent loop."
    case .ready:
      if !snapshot.isLocalModelReady {
        return "\(snapshot.modelSetupSummary) Use the Model chip or setup callout to continue."
      }

      if !snapshot.hasWorkspace {
        return "Use the Workspace chip or Command-O to bind tools to a local project."
      }

      if !snapshot.hasRuntimeThreadSelection {
        return "Use the Thread chip or Command-N to start local agent work."
      }

      if snapshot.hasActiveTurn {
        return "Pith is streaming locally. Cancel the turn if it is no longer useful."
      }

      if snapshot.isWaitingForFirstMessage {
        if snapshot.hasDraftMessage {
          return "Review the starter prompt, then press Command-Return to send the first local request."
        }
        return "Choose Map Workspace, Review Changes, or type a short local request."
      }

      return "Ready for local agent work. Press Command-Return to send."
    }
  }
}
