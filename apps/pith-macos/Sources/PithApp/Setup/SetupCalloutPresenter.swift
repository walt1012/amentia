import Foundation

struct SetupCalloutSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let isLocalModelReady: Bool
  let hasWorkspace: Bool
  let hasRuntimeThreadSelection: Bool
  let modelGuidance: LocalModelSetupGuidance
  let modelProgressDetail: String?
  let runtimeLaunchActionTitle: String
  let modelPrimaryActionTitle: String?
  let modelSecondaryActionTitle: String?
  let distributionTrustSetupDetail: String?
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
      return "Create Session"
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
      return "Create or select a session before starting the first cowork task."
    }

    return "Pith is ready for local work."
  }

  static func detail(_ snapshot: SetupCalloutSnapshot) -> String {
    if !snapshot.isLocalModelReady {
      return appendDistributionTrustDetail(
        snapshot.modelProgressDetail ?? snapshot.modelGuidance.detail,
        snapshot: snapshot
      )
    }
    if !snapshot.hasWorkspace {
      return "Workspace binding keeps file reads, search, shell actions, diffs, and memory scoped to one local project."
    }
    if !snapshot.hasRuntimeThreadSelection {
      return "Sessions keep messages, approvals, cancellation, and useful memory together."
    }

    return "Ready"
  }

  private static func appendDistributionTrustDetail(
    _ detail: String,
    snapshot: SetupCalloutSnapshot
  ) -> String {
    guard let trustDetail = snapshot.distributionTrustSetupDetail,
          snapshot.runtimeState == .disconnected || snapshot.runtimeState == .failed
    else {
      return detail
    }

    return "\(detail) \(trustDetail)"
  }

  static func tone(_ snapshot: SetupCalloutSnapshot) -> StatusTone {
    if !snapshot.isLocalModelReady {
      return snapshot.modelGuidance.tone
    }

    return .warning
  }

  static func primaryActionTitle(_ snapshot: SetupCalloutSnapshot) -> String? {
    if !snapshot.isLocalModelReady {
      if snapshot.runtimeState == .disconnected || snapshot.runtimeState == .failed {
        return snapshot.runtimeLaunchActionTitle
      }

      return snapshot.modelPrimaryActionTitle
    }
    if !snapshot.hasWorkspace {
      return "Open Workspace"
    }
    if !snapshot.hasRuntimeThreadSelection {
      return "New Session"
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
