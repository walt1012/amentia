import Foundation

struct RuntimeReadinessActionSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let isLocalModelReady: Bool
  let hasWorkspace: Bool
  let hasRuntimeThreadSelection: Bool
  let canLaunchRuntime: Bool
  let canRunModelSetupAction: Bool
  let canOpenWorkspace: Bool
  let canCreateThread: Bool
  let canUseComposer: Bool
  let isWaitingForFirstMessage: Bool
  let hasDraftMessage: Bool
  let hasFirstRequestSuggestion: Bool
  let runtimeReadinessChecks: [RuntimeReadinessCheckSummary]
  let canEnableWebSearchPlugin: Bool
  let runtimeLaunchButtonTitle: String
  let modelSetupActionTitle: String?
}

enum RuntimeReadinessAction {
  case launchRuntime
  case setupModel
  case openWorkspace
  case createThread
  case useFirstRequestPrompt
  case sendFirstRequest
  case enableWebSearchPlugin
  case openPluginAccess
  case openPluginCommands
  case inspectSandboxStatus
}

enum RuntimeReadinessActionPlanner {
  static func action(
    for step: ReadinessStepSummary,
    snapshot: RuntimeReadinessActionSnapshot
  ) -> RuntimeReadinessAction? {
    switch step.id {
    case "runtime":
      return runtimeAction(snapshot)
    case "model":
      return modelAction(snapshot)
    case "workspace":
      return workspaceAction(snapshot)
    case "thread":
      return threadAction(snapshot)
    case "first-request":
      return firstRequestAction(snapshot)
    case "tools":
      return toolsAction(snapshot)
    default:
      return nil
    }
  }

  static func title(
    for action: RuntimeReadinessAction?,
    snapshot: RuntimeReadinessActionSnapshot
  ) -> String? {
    guard let action else {
      return nil
    }

    switch action {
    case .launchRuntime:
      return snapshot.runtimeLaunchButtonTitle
    case .setupModel:
      return snapshot.modelSetupActionTitle
    case .openWorkspace:
      return "Open"
    case .createThread:
      return "New"
    case .useFirstRequestPrompt:
      return "Use Prompt"
    case .sendFirstRequest:
      return "Send"
    case .enableWebSearchPlugin:
      return "Enable"
    case .openPluginAccess:
      return "Access"
    case .openPluginCommands:
      return "Commands"
    case .inspectSandboxStatus:
      return "Inspect"
    }
  }

  static func canRun(
    _ action: RuntimeReadinessAction?,
    snapshot: RuntimeReadinessActionSnapshot
  ) -> Bool {
    guard let action else {
      return false
    }

    switch action {
    case .launchRuntime:
      return snapshot.canLaunchRuntime
    case .setupModel:
      return snapshot.canRunModelSetupAction
    case .openWorkspace:
      return snapshot.canOpenWorkspace
    case .createThread:
      return snapshot.canCreateThread
    case .useFirstRequestPrompt:
      return snapshot.hasFirstRequestSuggestion
    case .sendFirstRequest:
      return snapshot.hasDraftMessage
    case .enableWebSearchPlugin:
      return snapshot.canEnableWebSearchPlugin
    case .openPluginAccess, .openPluginCommands, .inspectSandboxStatus:
      return snapshot.runtimeState == .ready
    }
  }

  private static func runtimeAction(
    _ snapshot: RuntimeReadinessActionSnapshot
  ) -> RuntimeReadinessAction? {
    if snapshot.runtimeState == .disconnected || snapshot.runtimeState == .failed {
      return .launchRuntime
    }

    return nil
  }

  private static func modelAction(
    _ snapshot: RuntimeReadinessActionSnapshot
  ) -> RuntimeReadinessAction? {
    if snapshot.runtimeState == .ready && !snapshot.isLocalModelReady {
      return .setupModel
    }

    return nil
  }

  private static func workspaceAction(
    _ snapshot: RuntimeReadinessActionSnapshot
  ) -> RuntimeReadinessAction? {
    if snapshot.runtimeState == .ready && !snapshot.hasWorkspace {
      return .openWorkspace
    }

    return nil
  }

  private static func threadAction(
    _ snapshot: RuntimeReadinessActionSnapshot
  ) -> RuntimeReadinessAction? {
    if snapshot.runtimeState == .ready
      && snapshot.isLocalModelReady
      && snapshot.hasWorkspace
      && !snapshot.hasRuntimeThreadSelection
    {
      return .createThread
    }

    return nil
  }

  private static func firstRequestAction(
    _ snapshot: RuntimeReadinessActionSnapshot
  ) -> RuntimeReadinessAction? {
    guard snapshot.runtimeState == .ready,
          snapshot.canUseComposer,
          snapshot.isWaitingForFirstMessage
    else {
      return nil
    }

    return snapshot.hasDraftMessage ? .sendFirstRequest : .useFirstRequestPrompt
  }

  private static func toolsAction(
    _ snapshot: RuntimeReadinessActionSnapshot
  ) -> RuntimeReadinessAction? {
    guard snapshot.runtimeState == .ready else {
      return nil
    }

    switch RuntimeToolReadinessPresenter.primaryIssueID(snapshot.runtimeReadinessChecks) {
    case "webSearch":
      if snapshot.canEnableWebSearchPlugin {
        return .enableWebSearchPlugin
      }
      return .openPluginAccess
    case "plugins":
      return .openPluginCommands
    case "nativeSandbox":
      return .inspectSandboxStatus
    default:
      return nil
    }
  }
}
