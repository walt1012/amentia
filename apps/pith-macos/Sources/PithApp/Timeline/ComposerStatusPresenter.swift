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
  let hasRestoredLocalExecutionDraft: Bool
}

enum ComposerStatusPresenter {
  static func placeholder(_ snapshot: ComposerStatusSnapshot) -> String {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Start the local service to begin"
    case .launching:
      return "Local service is starting..."
    case .failed:
      return "Restart the local service to recover"
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
      return "Create or select a session"
    }

    if snapshot.hasActiveTurn {
      return "Pith is running a local execution. Cancel to stop it."
    }

    if snapshot.hasRestoredLocalExecutionDraft {
      return "Review the restored prompt, then send"
    }

    if snapshot.isWaitingForFirstMessage {
      return snapshot.hasDraftMessage
        ? "Review the first cowork prompt, then send"
        : "Choose a starter prompt or type the first cowork prompt"
    }

    return "Ask Pith to inspect files, review diffs, or make a safe local change"
  }

  static func statusSummary(_ snapshot: ComposerStatusSnapshot) -> String {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Start the local service to begin."
    case .launching:
      return "Starting the local service..."
    case .failed:
      return "Restart the local service to recover."
    case .ready:
      if !snapshot.isLocalModelReady {
        return "\(snapshot.modelSetupSummary) Continue model setup to run locally."
      }

      if !snapshot.hasWorkspace {
        return "Open a workspace to bind tools to a local project."
      }

      if !snapshot.hasRuntimeThreadSelection {
        return "Create a session to start local work."
      }

      if snapshot.hasActiveTurn {
        return "Pith is running locally. Cancel the execution if it is no longer useful."
      }

      if snapshot.hasRestoredLocalExecutionDraft {
        return "Ask mode is ready. Review the restored prompt, then send it."
      }

      if snapshot.isWaitingForFirstMessage {
        if snapshot.hasDraftMessage {
          return "Review the starter prompt, then start the cowork session."
        }
        return "Choose a starter prompt or type a short cowork prompt."
      }

      return "Ready for local work."
    }
  }
}
