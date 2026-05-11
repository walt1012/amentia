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

struct PreparedDraftTurn {
  let threadID: String
  let message: String
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

  static func runtimePrimaryActionTitle(_ snapshot: SessionActionSnapshot) -> String? {
    canCancelActiveTurn(snapshot) ? "Cancel Execution" : nil
  }

  static func canRunRuntimePrimaryAction(_ snapshot: SessionActionSnapshot) -> Bool {
    canCancelActiveTurn(snapshot)
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

  static func preparedDraftTurn(
    snapshot: SessionActionSnapshot,
    selectedThreadID: ThreadSummary.ID?,
    draftMessage: String
  ) -> PreparedDraftTurn? {
    let message = draftMessage.trimmingCharacters(in: .whitespacesAndNewlines)

    guard canSendDraftMessage(snapshot),
          let threadID = selectedThreadID,
          !threadID.hasPrefix("local-"),
          !message.isEmpty
    else {
      return nil
    }

    return PreparedDraftTurn(threadID: threadID, message: message)
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
