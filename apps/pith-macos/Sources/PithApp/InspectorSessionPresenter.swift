import Foundation

struct InspectorSessionSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let isLocalModelReady: Bool
  let hasWorkspace: Bool
  let workspaceDisplayName: String?
  let hasRuntimeThreadSelection: Bool
  let selectedThreadTitle: String
  let hasActiveTurn: Bool
  let setupReadyStepCount: Int
  let setupStepCount: Int
  let setupProgressDetail: String
  let isWaitingForFirstMessage: Bool
}

enum InspectorSessionPresenter {
  static func title(_ snapshot: InspectorSessionSnapshot) -> String {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Local Runtime Offline"
    case .launching:
      return "Starting Local Runtime"
    case .failed:
      return "Runtime Needs Relaunch"
    case .ready:
      if snapshot.hasActiveTurn {
        return "Local Turn Running"
      }
      if !snapshot.isLocalModelReady {
        return "Model Setup Needed"
      }
      if !snapshot.hasWorkspace {
        return "Workspace Needed"
      }
      if !snapshot.hasRuntimeThreadSelection {
        return "Thread Needed"
      }
      return "Local Session Ready"
    }
  }

  static func detail(_ snapshot: InspectorSessionSnapshot) -> String {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Launch Pith's runtime before inspecting project tools, model state, memory, or plugins."
    case .launching:
      return "Pith is reconnecting local model, workspace, thread, memory, and plugin state."
    case .failed:
      return "Use the relaunch action in the timeline header to recover the local agent loop."
    case .ready:
      if snapshot.hasActiveTurn {
        return "Pith is streaming locally. Keep review focused on the timeline unless the turn should be cancelled."
      }
      if !snapshot.isLocalModelReady {
        return "Complete the model step from the timeline callout before starting agent work."
      }
      if !snapshot.hasWorkspace {
        return "Open one workspace so file, shell, search, diff, and memory actions stay scoped."
      }
      if !snapshot.hasRuntimeThreadSelection {
        return "Create or select a thread to keep messages, approvals, memory, and cancellation together."
      }
      return "Use the composer for the next request. Open inspector sections only when detail is needed."
    }
  }

  static func metaSummary(_ snapshot: InspectorSessionSnapshot) -> String {
    if snapshot.setupReadyStepCount < snapshot.setupStepCount
      || snapshot.hasActiveTurn
      || snapshot.isWaitingForFirstMessage
    {
      return snapshot.setupProgressDetail
    }

    let modelSummary = snapshot.isLocalModelReady ? "Model ready" : "Model pending"
    let workspaceSummary = snapshot.workspaceDisplayName ?? "No workspace"
    let threadSummary = snapshot.hasRuntimeThreadSelection ? snapshot.selectedThreadTitle : "No thread"
    return "\(modelSummary) | \(workspaceSummary) | \(threadSummary)"
  }
}
