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
      return "Local Runtime Offline"
    case .launching:
      return "Starting Local Runtime"
    case .failed:
      return "Runtime Needs Relaunch"
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
        return "Thread Needed"
      }
      if snapshot.isWaitingForFirstMessage {
        return "First Request Ready"
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
      return "Use the relaunch action in the timeline header to recover the local runtime."
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
        return "Create or select a thread to keep messages, approvals, memory, and cancellation together."
      }
      if snapshot.isWaitingForFirstMessage {
        return "Send one short cowork request from the composer to finish first-use setup."
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
    let threadSummary = snapshot.hasRuntimeThreadSelection ? snapshot.selectedThreadTitle : "No thread"
    return [readinessSummary, toolSummary, selectedModeSummary, workspaceSummary, threadSummary]
      .compactMap { $0 }
      .joined(separator: " | ")
  }

  private static func runtimeReadinessSummary(_ status: String) -> String {
    switch status {
    case "ready":
      return "Runtime ready"
    case "running":
      return "Runtime running"
    case "needs_approval":
      return "Runtime needs approval"
    case "setup_required":
      return "Runtime setup"
    default:
      return "Runtime \(status)"
    }
  }

}
