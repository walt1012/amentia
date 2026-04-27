import Foundation

struct SetupProgressSnapshot {
  let readyStepCount: Int
  let stepCount: Int
  let runtimeState: RuntimeBridge.ConnectionState
  let showsRuntimeActivity: Bool
  let isLocalModelReady: Bool
  let hasWorkspace: Bool
  let hasRuntimeThreadSelection: Bool
  let hasActiveTurn: Bool
  let isWaitingForFirstMessage: Bool
  let hasDraft: Bool
  let modelReadinessDetail: String
}

enum SetupProgressPresenter {
  static func summary(_ snapshot: SetupProgressSnapshot) -> String {
    if snapshot.readyStepCount == snapshot.stepCount {
      return "Local setup complete"
    }

    return "Local setup \(snapshot.readyStepCount)/\(snapshot.stepCount)"
  }

  static func detail(_ snapshot: SetupProgressSnapshot) -> String {
    if snapshot.hasActiveTurn {
      return "Turn running"
    }
    if let nextStep = nextStep(snapshot) {
      return "Next: \(nextStep)"
    }
    if snapshot.isWaitingForFirstMessage {
      return snapshot.hasDraft ? "Draft ready" : "Next: First request"
    }

    return "Ready"
  }

  static func value(_ snapshot: SetupProgressSnapshot) -> Double {
    guard snapshot.stepCount > 0 else {
      return 0
    }

    return Double(snapshot.readyStepCount) / Double(snapshot.stepCount)
  }

  static func tone(_ snapshot: SetupProgressSnapshot) -> StatusTone {
    if snapshot.runtimeState == .failed {
      return .danger
    }
    if snapshot.showsRuntimeActivity {
      return .active
    }

    return snapshot.readyStepCount == snapshot.stepCount ? .ready : .warning
  }

  private static func nextStep(_ snapshot: SetupProgressSnapshot) -> String? {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Launch Runtime"
    case .launching:
      return "Runtime Starting"
    case .failed:
      return "Relaunch Runtime"
    case .ready:
      if !snapshot.isLocalModelReady {
        return modelNextStep(snapshot.modelReadinessDetail)
      }
      if !snapshot.hasWorkspace {
        return "Open Workspace"
      }
      if !snapshot.hasRuntimeThreadSelection {
        return "Create Thread"
      }
      if snapshot.isWaitingForFirstMessage {
        return snapshot.hasDraft ? "Send First Request" : "Choose First Prompt"
      }

      return nil
    }
  }

  private static func modelNextStep(_ readinessDetail: String) -> String {
    switch readinessDetail {
    case "Downloading":
      return "Monitor Model"
    case "Paused":
      return "Continue Download"
    case "Blocked":
      return "Free Model Space"
    case "Streaming":
      return "Finish Turn"
    case "Select":
      return "Use Model"
    case "Metadata":
      return "Install Metadata"
    default:
      return "Download Model"
    }
  }
}
