import Foundation

struct SetupCalloutActionSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let isLocalModelReady: Bool
  let hasWorkspace: Bool
  let hasRuntimeThreadSelection: Bool
  let canLaunchRuntime: Bool
  let canRunModelSetupAction: Bool
  let canRunModelSetupSecondaryAction: Bool
  let canOpenWorkspace: Bool
  let canCreateThread: Bool
}

enum SetupCalloutAction {
  case launchRuntime
  case setupModel
  case openWorkspace
  case createThread
}

enum SetupCalloutSecondaryAction {
  case setupModelSecondary
}

enum SetupCalloutActionPlanner {
  static func primaryAction(_ snapshot: SetupCalloutActionSnapshot) -> SetupCalloutAction? {
    if !snapshot.isLocalModelReady {
      if snapshot.runtimeState == .disconnected || snapshot.runtimeState == .failed {
        return .launchRuntime
      }

      return .setupModel
    }
    if !snapshot.hasWorkspace {
      return .openWorkspace
    }
    if !snapshot.hasRuntimeThreadSelection {
      return .createThread
    }

    return nil
  }

  static func secondaryAction(
    _ snapshot: SetupCalloutActionSnapshot
  ) -> SetupCalloutSecondaryAction? {
    snapshot.isLocalModelReady ? nil : .setupModelSecondary
  }

  static func canRun(
    _ action: SetupCalloutAction?,
    snapshot: SetupCalloutActionSnapshot
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
    }
  }

  static func canRun(
    _ action: SetupCalloutSecondaryAction?,
    snapshot: SetupCalloutActionSnapshot
  ) -> Bool {
    guard let action else {
      return false
    }

    switch action {
    case .setupModelSecondary:
      return snapshot.canRunModelSetupSecondaryAction
    }
  }
}
