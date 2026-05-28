import Foundation

struct RuntimeReadinessSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let modelReadinessDetail: String
  let modelTone: StatusTone
  let workspaceDisplayName: String?
  let isLocalModelReady: Bool
  let hasWorkspace: Bool
  let hasRuntimeThreadSelection: Bool
  let hasActiveTurn: Bool
  let isWaitingForFirstMessage: Bool
  let hasDraftMessage: Bool
  let runtimeReadinessChecks: [RuntimeReadinessCheckSummary]
}

struct ReadinessStepSummary: Identifiable, Hashable {
  let id: String
  let label: String
  let detail: String
  let tone: StatusTone
}

enum RuntimeReadinessPresenter {
  static func steps(_ snapshot: RuntimeReadinessSnapshot) -> [ReadinessStepSummary] {
    [
      runtimeStep(snapshot),
      modelStep(snapshot),
      workspaceStep(snapshot),
      threadStep(snapshot),
      firstRequestStep(snapshot),
      toolsStep(snapshot),
    ]
  }

  private static func runtimeStep(_ snapshot: RuntimeReadinessSnapshot) -> ReadinessStepSummary {
    switch snapshot.runtimeState {
    case .ready:
      return ReadinessStepSummary(id: "runtime", label: "Runtime", detail: "Ready", tone: .ready)
    case .launching:
      return ReadinessStepSummary(id: "runtime", label: "Runtime", detail: "Starting", tone: .active)
    case .failed:
      return ReadinessStepSummary(id: "runtime", label: "Runtime", detail: "Relaunch", tone: .danger)
    case .disconnected:
      return ReadinessStepSummary(id: "runtime", label: "Runtime", detail: "Launch", tone: .warning)
    }
  }

  private static func modelStep(_ snapshot: RuntimeReadinessSnapshot) -> ReadinessStepSummary {
    guard snapshot.runtimeState == .ready else {
      return ReadinessStepSummary(id: "model", label: "Model", detail: "Waiting", tone: .neutral)
    }

    return ReadinessStepSummary(
      id: "model",
      label: "Model",
      detail: snapshot.modelReadinessDetail,
      tone: snapshot.modelTone
    )
  }

  private static func workspaceStep(_ snapshot: RuntimeReadinessSnapshot) -> ReadinessStepSummary {
    guard snapshot.runtimeState == .ready else {
      return ReadinessStepSummary(id: "workspace", label: "Workspace", detail: "Waiting", tone: .neutral)
    }
    guard let workspaceDisplayName = snapshot.workspaceDisplayName else {
      return ReadinessStepSummary(id: "workspace", label: "Workspace", detail: "Open", tone: .warning)
    }

    return ReadinessStepSummary(
      id: "workspace",
      label: "Workspace",
      detail: workspaceDisplayName,
      tone: .ready
    )
  }

  private static func threadStep(_ snapshot: RuntimeReadinessSnapshot) -> ReadinessStepSummary {
    guard snapshot.runtimeState == .ready else {
      return ReadinessStepSummary(id: "thread", label: "Thread", detail: "Waiting", tone: .neutral)
    }
    guard snapshot.isLocalModelReady, snapshot.hasWorkspace else {
      return ReadinessStepSummary(id: "thread", label: "Thread", detail: "Waiting", tone: .neutral)
    }
    guard snapshot.hasRuntimeThreadSelection else {
      return ReadinessStepSummary(id: "thread", label: "Thread", detail: "Create", tone: .warning)
    }
    if snapshot.hasActiveTurn {
      return ReadinessStepSummary(id: "thread", label: "Thread", detail: "Streaming", tone: .active)
    }

    return ReadinessStepSummary(id: "thread", label: "Thread", detail: "Ready", tone: .ready)
  }

  private static func firstRequestStep(_ snapshot: RuntimeReadinessSnapshot) -> ReadinessStepSummary {
    guard snapshot.runtimeState == .ready else {
      return ReadinessStepSummary(
        id: "first-request",
        label: "First Request",
        detail: "Waiting",
        tone: .neutral
      )
    }
    guard snapshot.isLocalModelReady,
          snapshot.hasWorkspace,
          snapshot.hasRuntimeThreadSelection
    else {
      return ReadinessStepSummary(
        id: "first-request",
        label: "First Request",
        detail: "Waiting",
        tone: .neutral
      )
    }
    if snapshot.hasActiveTurn {
      return ReadinessStepSummary(
        id: "first-request",
        label: "First Request",
        detail: "Running",
        tone: .active
      )
    }
    guard snapshot.isWaitingForFirstMessage else {
      return ReadinessStepSummary(
        id: "first-request",
        label: "First Request",
        detail: "Sent",
        tone: .ready
      )
    }
    if snapshot.hasDraftMessage {
      return ReadinessStepSummary(
        id: "first-request",
        label: "First Request",
        detail: "Draft",
        tone: .warning
      )
    }

    return ReadinessStepSummary(
      id: "first-request",
      label: "First Request",
      detail: "Prompt",
      tone: .warning
    )
  }

  private static func toolsStep(_ snapshot: RuntimeReadinessSnapshot) -> ReadinessStepSummary {
    guard snapshot.runtimeState == .ready else {
      return ReadinessStepSummary(id: "tools", label: "Tools", detail: "Waiting", tone: .neutral)
    }

    guard RuntimeToolReadinessPresenter.hasToolChecks(snapshot.runtimeReadinessChecks) else {
      return ReadinessStepSummary(id: "tools", label: "Tools", detail: "Waiting", tone: .neutral)
    }

    return ReadinessStepSummary(
      id: "tools",
      label: "Tools",
      detail: RuntimeToolReadinessPresenter.timelineDetail(snapshot.runtimeReadinessChecks),
      tone: RuntimeToolReadinessPresenter.timelineTone(snapshot.runtimeReadinessChecks)
    )
  }
}

enum DailyDriverStagePresenter {
  static func summary(stage: String?, nextAction: String?) -> String? {
    if let nextAction = cleaned(nextAction) {
      return nextAction
    }

    guard let stage = cleaned(stage) else {
      return nil
    }

    switch stage {
    case "model_setup":
      return "Download and select a verified local model."
    case "workspace_setup":
      return "Open a workspace to scope tools and memory."
    case "thread_setup":
      return "Create or select a workspace-bound thread."
    case "approval_review":
      return "Review the pending approval before work continues."
    case "local_execution":
      return "Wait for local work or cancel it if it is no longer useful."
    case "first_request":
      return "Send the first cowork request."
    case "ready":
      return "Ask Pith for the next cowork task."
    default:
      return nil
    }
  }

  static func tone(stage: String?) -> StatusTone {
    switch cleaned(stage) {
    case "ready":
      return .ready
    case "local_execution":
      return .active
    case "model_setup", "workspace_setup", "thread_setup", "approval_review", "first_request":
      return .warning
    default:
      return .neutral
    }
  }

  private static func cleaned(_ value: String?) -> String? {
    guard let trimmed = value?.trimmingCharacters(in: .whitespacesAndNewlines),
          !trimmed.isEmpty
    else {
      return nil
    }
    return trimmed
  }
}
