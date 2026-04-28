import Foundation

struct SetupCalloutActionSnapshot {
  let isLocalModelReady: Bool
  let hasWorkspace: Bool
  let hasRuntimeThreadSelection: Bool
  let canRunModelSetupAction: Bool
  let canRunModelSetupSecondaryAction: Bool
  let canOpenWorkspace: Bool
  let canCreateThread: Bool
}

enum SetupCalloutAction {
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
