import Foundation

struct SetupCalloutSnapshot {
  let isLocalModelReady: Bool
  let hasWorkspace: Bool
  let hasRuntimeThreadSelection: Bool
  let modelGuidance: LocalModelSetupGuidance
  let modelProgressDetail: String?
  let modelPrimaryActionTitle: String?
  let modelSecondaryActionTitle: String?
}

enum SetupCalloutPresenter {
  static func title(_ snapshot: SetupCalloutSnapshot) -> String {
    if !snapshot.isLocalModelReady {
      return snapshot.modelGuidance.title
    }
    if !snapshot.hasWorkspace {
      return "Open Workspace"
    }
    if !snapshot.hasRuntimeThreadSelection {
      return "Create Thread"
    }

    return "Local Setup Complete"
  }

  static func summary(_ snapshot: SetupCalloutSnapshot) -> String {
    if !snapshot.isLocalModelReady {
      return snapshot.modelGuidance.summary
    }
    if !snapshot.hasWorkspace {
      return "Choose the project Pith should inspect, search, and edit locally."
    }
    if !snapshot.hasRuntimeThreadSelection {
      return "Create or select a runtime thread before sending the first local request."
    }

    return "Pith is ready for local agent work."
  }

  static func detail(_ snapshot: SetupCalloutSnapshot) -> String {
    if !snapshot.isLocalModelReady {
      return snapshot.modelProgressDetail ?? snapshot.modelGuidance.detail
    }
    if !snapshot.hasWorkspace {
      return "Workspace binding keeps file reads, search, shell actions, diffs, and memory scoped to one local project."
    }
    if !snapshot.hasRuntimeThreadSelection {
      return "Threads keep the timeline, approvals, cancellation, and memory context together."
    }

    return "Ready"
  }

  static func tone(_ snapshot: SetupCalloutSnapshot) -> StatusTone {
    if !snapshot.isLocalModelReady {
      return snapshot.modelGuidance.tone
    }

    return .warning
  }

  static func primaryActionTitle(_ snapshot: SetupCalloutSnapshot) -> String? {
    if !snapshot.isLocalModelReady {
      return snapshot.modelPrimaryActionTitle
    }
    if !snapshot.hasWorkspace {
      return "Open Workspace"
    }
    if !snapshot.hasRuntimeThreadSelection {
      return "New Thread"
    }

    return nil
  }

  static func secondaryActionTitle(_ snapshot: SetupCalloutSnapshot) -> String? {
    if !snapshot.isLocalModelReady {
      return snapshot.modelSecondaryActionTitle
    }

    return nil
  }
}
