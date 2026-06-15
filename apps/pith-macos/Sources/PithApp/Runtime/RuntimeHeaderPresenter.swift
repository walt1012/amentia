import Foundation

struct RuntimeHeaderSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let runtimeDetail: String
  let modelSetupSummary: String
  let isLocalModelReady: Bool
  let hasWorkspace: Bool
  let hasRuntimeThreadSelection: Bool
  let hasActiveTurn: Bool
  let isWaitingForFirstMessage: Bool
  let hasDraftMessage: Bool
  let isWorkspaceSearching: Bool
  let hasModelDownload: Bool
  let hasPausedModelDownload: Bool
  let hasToolReadinessIssue: Bool
  let dailyDriverStage: String?
  let dailyDriverNextAction: String?
}

enum RuntimeHeaderPresenter {
  static func statusSummary(_ snapshot: RuntimeHeaderSnapshot) -> String {
    switch snapshot.runtimeState {
    case .disconnected:
      return "Start Pith to restore model, project, connections, and memory."
    case .launching:
      return "Starting Pith and reconnecting local state..."
    case .failed:
      return "Pith stopped. Restart to recover local work."
    case .ready:
      if !snapshot.isLocalModelReady {
        return snapshot.modelSetupSummary
      }
      if !snapshot.hasWorkspace {
        return DailyDriverStagePresenter.summary(
          stage: snapshot.dailyDriverStage,
          nextAction: snapshot.dailyDriverNextAction
        ) ?? "Model is ready. Open a project to bind tools to local files."
      }
      if snapshot.hasActiveTurn {
        return DailyDriverStagePresenter.summary(
          stage: snapshot.dailyDriverStage,
          nextAction: snapshot.dailyDriverNextAction
        ) ?? "Pith is running locally. Cancel only if the execution is no longer useful."
      }
      if !snapshot.hasRuntimeThreadSelection {
        return DailyDriverStagePresenter.summary(
          stage: snapshot.dailyDriverStage,
          nextAction: snapshot.dailyDriverNextAction
        ) ?? "Select or create a session to start local work."
      }
      if snapshot.isWaitingForFirstMessage {
        return snapshot.hasDraftMessage
          ? "First cowork prompt is drafted. Send it to finish setup."
          : DailyDriverStagePresenter.summary(
            stage: snapshot.dailyDriverStage,
            nextAction: snapshot.dailyDriverNextAction
          ) ?? FirstRequestPromptPresenter.firstAppOpenActionSummary()
      }
      return DailyDriverStagePresenter.summary(
        stage: snapshot.dailyDriverStage,
        nextAction: snapshot.dailyDriverNextAction
      ) ?? "Ready for local work."
    }
  }

  static func statusTone(_ snapshot: RuntimeHeaderSnapshot) -> StatusTone {
    switch snapshot.runtimeState {
    case .disconnected:
      return .warning
    case .launching:
      return .active
    case .failed:
      return .danger
    case .ready:
      if snapshot.hasActiveTurn || snapshot.hasModelDownload || snapshot.isWorkspaceSearching {
        return .active
      }
      let dailyTone = DailyDriverStagePresenter.tone(stage: snapshot.dailyDriverStage)
      if dailyTone != .neutral {
        return dailyTone
      }
      if !snapshot.hasWorkspace
        || !snapshot.isLocalModelReady
        || !snapshot.hasRuntimeThreadSelection
        || snapshot.isWaitingForFirstMessage
      {
        return .warning
      }
      return .ready
    }
  }

  static func showsActivity(_ snapshot: RuntimeHeaderSnapshot) -> Bool {
    snapshot.runtimeState == .launching
      || snapshot.isWorkspaceSearching
      || snapshot.hasModelDownload
      || snapshot.hasActiveTurn
  }

  static func shouldShowDetail(_ snapshot: RuntimeHeaderSnapshot) -> Bool {
    guard !snapshot.runtimeDetail.isEmpty else {
      return false
    }

    switch snapshot.runtimeState {
    case .disconnected:
      return snapshot.runtimeDetail != "Pith not started"
    case .launching, .failed:
      return true
    case .ready:
      return snapshot.hasActiveTurn
        || snapshot.hasModelDownload
        || snapshot.hasPausedModelDownload
        || snapshot.isWorkspaceSearching
        || !snapshot.isLocalModelReady
        || !snapshot.hasWorkspace
        || !snapshot.hasRuntimeThreadSelection
        || snapshot.isWaitingForFirstMessage
        || snapshot.hasToolReadinessIssue
    }
  }
}
