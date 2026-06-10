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
  let runtimeReadinessStatus: String?
  let dailyDriverStage: String?
  let dailyDriverNextAction: String?
  let runtimeReadinessChecks: [RuntimeReadinessCheckSummary]
  let runtimeReadinessMetrics: [String: String]
  let selectedLocalExecutionSafetyMode: String
}

enum InspectorSessionPresenter {
  static func title(_ snapshot: InspectorSessionSnapshot) -> String {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Local Service Offline"
    case .launching:
      return "Starting Local Service"
    case .failed:
      return "Local Service Needs Restart"
    case .ready:
      if snapshot.hasActiveTurn {
        return "Local Execution Running"
      }
      if !snapshot.isLocalModelReady {
        return "Model Setup Needed"
      }
      if !snapshot.hasWorkspace {
        return "Workspace Needed"
      }
      if !snapshot.hasRuntimeThreadSelection {
        return "Session Needed"
      }
      if snapshot.isWaitingForFirstMessage {
        return "First Prompt Ready"
      }
      return "Local Session Ready"
    }
  }

  static func detail(_ snapshot: InspectorSessionSnapshot) -> String {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Start Pith's local service before inspecting project actions, model state, memory, or connectors."
    case .launching:
      return "Pith is reconnecting local model, workspace, session, memory, and connector state."
    case .failed:
      return "Use the restart action in the timeline header to recover the local service."
    case .ready:
      if snapshot.hasActiveTurn {
        return "Pith is running locally. Keep review focused on the timeline unless the execution should be cancelled."
      }
      if !snapshot.isLocalModelReady {
        return "Complete the model step from the timeline callout before starting agent work."
      }
      if !snapshot.hasWorkspace {
        return "Open one workspace so file, shell, search, diff, and memory actions stay scoped."
      }
      if !snapshot.hasRuntimeThreadSelection {
        return "Create or select a session to keep messages, approvals, memory, and cancellation together."
      }
      if snapshot.isWaitingForFirstMessage {
        return "Send one short cowork prompt from the composer to finish first-use setup."
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
    let readinessSummary = DailyDriverStagePresenter.summary(
      stage: snapshot.dailyDriverStage,
      nextAction: snapshot.dailyDriverNextAction
    ) ?? snapshot.runtimeReadinessStatus.map(runtimeReadinessSummary) ?? modelSummary
    let toolSummary = RuntimeToolReadinessPresenter.inspectorSummary(
      snapshot.runtimeReadinessChecks,
      metrics: snapshot.runtimeReadinessMetrics
    )
    let selectedModeSummary = "Mode "
      + LocalExecutionSafetyModePresenter.compact(snapshot.selectedLocalExecutionSafetyMode)
    let workspaceSummary = snapshot.workspaceDisplayName ?? "No workspace"
    let threadSummary = snapshot.hasRuntimeThreadSelection ? snapshot.selectedThreadTitle : "No session"
    return [readinessSummary, toolSummary, selectedModeSummary, workspaceSummary, threadSummary]
      .compactMap { $0 }
      .joined(separator: " | ")
  }

  private static func runtimeReadinessSummary(_ status: String) -> String {
    switch status {
    case "ready":
      return "Local service ready"
    case "running":
      return "Local work running"
    case "needs_approval":
      return "Approval needed"
    case "setup_required":
      return "Setup needed"
    default:
      return "Local service \(status)"
    }
  }

}
