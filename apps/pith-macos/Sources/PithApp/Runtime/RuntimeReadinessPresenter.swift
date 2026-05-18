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

    let toolChecks = runtimeToolChecks(snapshot.runtimeReadinessChecks)
    guard !toolChecks.isEmpty else {
      return ReadinessStepSummary(id: "tools", label: "Tools", detail: "Waiting", tone: .neutral)
    }

    guard let issue = toolChecks.first(where: { !isReadyToolStatus($0.status) }) else {
      return ReadinessStepSummary(id: "tools", label: "Tools", detail: "Ready", tone: .ready)
    }

    return ReadinessStepSummary(
      id: "tools",
      label: "Tools",
      detail: toolReadinessDetail(issue),
      tone: toolReadinessTone(issue.status)
    )
  }

  private static func runtimeToolChecks(
    _ checks: [RuntimeReadinessCheckSummary]
  ) -> [RuntimeReadinessCheckSummary] {
    let toolIDs = ["webSearch", "nativeSandbox", "plugins"]
    return toolIDs.compactMap { id in
      checks.first(where: { $0.id == id })
    }
  }

  private static func isReadyToolStatus(_ status: String) -> Bool {
    status == "ready"
  }

  private static func toolReadinessDetail(_ check: RuntimeReadinessCheckSummary) -> String {
    let label: String
    switch check.id {
    case "webSearch":
      label = "Web"
    case "nativeSandbox":
      label = "Sandbox"
    case "plugins":
      label = "Plugins"
    default:
      label = check.title
    }

    return "\(label) \(readinessStatusTitle(check.status))"
  }

  private static func readinessStatusTitle(_ status: String) -> String {
    switch status {
    case "limited":
      return "Limited"
    case "optional":
      return "Optional"
    case "setup_required":
      return "Setup"
    case "running":
      return "Running"
    case "needs_approval":
      return "Approval"
    case "failed":
      return "Failed"
    case "blocked":
      return "Blocked"
    default:
      return status.capitalized
    }
  }

  private static func toolReadinessTone(_ status: String) -> StatusTone {
    switch status {
    case "running":
      return .active
    case "limited", "optional", "setup_required", "needs_approval":
      return .warning
    default:
      return .danger
    }
  }
}
