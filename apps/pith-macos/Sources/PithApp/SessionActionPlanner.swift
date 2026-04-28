import Foundation

struct SessionActionSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let hasWorkspace: Bool
  let isLocalModelReady: Bool
  let hasRuntimeThreadSelection: Bool
  let hasActiveOrPendingTurn: Bool
  let hasCancelableTurn: Bool
  let hasDraftMessage: Bool
  let pendingApprovalIDs: Set<String>
}

enum RuntimePrimaryAction {
  case launchRuntime
  case cancelTurn
}

enum SessionActionPlanner {
  static func runtimeLaunchButtonTitle(_ snapshot: SessionActionSnapshot) -> String {
    switch snapshot.runtimeState {
    case .ready, .failed:
      return "Relaunch Runtime"
    case .launching:
      return "Launching Runtime"
    case .disconnected:
      return "Launch Runtime"
    }
  }

  static func shouldShowRuntimeToolbarAction(_ snapshot: SessionActionSnapshot) -> Bool {
    snapshot.runtimeState == .disconnected || snapshot.runtimeState == .failed
  }

  static func runtimePrimaryAction(_ snapshot: SessionActionSnapshot) -> RuntimePrimaryAction? {
    switch snapshot.runtimeState {
    case .disconnected, .failed, .launching:
      return .launchRuntime
    case .ready:
      return snapshot.hasCancelableTurn ? .cancelTurn : nil
    }
  }

  static func runtimePrimaryActionTitle(
    for action: RuntimePrimaryAction?,
    snapshot: SessionActionSnapshot
  ) -> String? {
    guard let action else {
      return nil
    }

    switch action {
    case .launchRuntime:
      return runtimeLaunchButtonTitle(snapshot)
    case .cancelTurn:
      return "Cancel Turn"
    }
  }

  static func canRunRuntimePrimaryAction(
    _ action: RuntimePrimaryAction?,
    snapshot: SessionActionSnapshot
  ) -> Bool {
    guard let action else {
      return false
    }

    switch action {
    case .launchRuntime:
      return canLaunchRuntime(snapshot)
    case .cancelTurn:
      return canCancelActiveTurn(snapshot)
    }
  }

  static func canLaunchRuntime(_ snapshot: SessionActionSnapshot) -> Bool {
    snapshot.runtimeState != .launching
  }

  static func canOpenWorkspace(_ snapshot: SessionActionSnapshot) -> Bool {
    snapshot.runtimeState == .ready && !snapshot.hasActiveOrPendingTurn
  }

  static func canCreateThread(_ snapshot: SessionActionSnapshot) -> Bool {
    snapshot.runtimeState == .ready
      && snapshot.hasWorkspace
      && snapshot.isLocalModelReady
      && !snapshot.hasActiveOrPendingTurn
  }

  static func canInstallPlugin(_ snapshot: SessionActionSnapshot) -> Bool {
    snapshot.runtimeState == .ready && !snapshot.hasActiveOrPendingTurn
  }

  static func canSendDraftMessage(_ snapshot: SessionActionSnapshot) -> Bool {
    snapshot.runtimeState == .ready
      && snapshot.hasWorkspace
      && snapshot.isLocalModelReady
      && snapshot.hasRuntimeThreadSelection
      && !snapshot.hasActiveOrPendingTurn
      && snapshot.hasDraftMessage
  }

  static func canCancelActiveTurn(_ snapshot: SessionActionSnapshot) -> Bool {
    snapshot.runtimeState == .ready && snapshot.hasCancelableTurn
  }

  static func canRespondToApproval(
    approvalID: String,
    snapshot: SessionActionSnapshot
  ) -> Bool {
    snapshot.runtimeState == .ready
      && snapshot.hasRuntimeThreadSelection
      && snapshot.pendingApprovalIDs.contains(approvalID)
  }

  static func canUseComposer(_ snapshot: SessionActionSnapshot) -> Bool {
    snapshot.runtimeState == .ready
      && snapshot.hasWorkspace
      && snapshot.isLocalModelReady
      && snapshot.hasRuntimeThreadSelection
      && !snapshot.hasActiveOrPendingTurn
  }
}
