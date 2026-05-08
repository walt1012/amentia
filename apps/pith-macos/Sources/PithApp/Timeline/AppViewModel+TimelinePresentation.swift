import Foundation

@MainActor
extension AppViewModel {
  func selectedThreadTitle() -> String {
    SessionOverviewPresenter.selectedThreadTitle(sessionOverviewSnapshot())
  }

  func selectedThreadPreview() -> String {
    SessionOverviewPresenter.selectedThreadPreview(sessionOverviewSnapshot())
  }

  func selectTimelineEntry(id: TimelineEntry.ID) {
    selectedEntryID = id
  }

  func selectedEntryTitle() -> String {
    TimelineInspectorPresenter.selectedEntryTitle(timelineInspectorSnapshot())
  }

  func selectedEntryBody() -> String {
    TimelineInspectorPresenter.selectedEntryBody(timelineInspectorSnapshot())
  }

  func selectedEntryMetadata() -> String {
    TimelineInspectorPresenter.selectedEntryMetadata(timelineInspectorSnapshot())
  }

  func selectedDiffSummary() -> String? {
    TimelineInspectorPresenter.selectedDiffSummary(timelineInspectorSnapshot())
  }

  func selectedDiffLines() -> [DiffLineSummary] {
    TimelineInspectorPresenter.selectedDiffLines(timelineInspectorSnapshot())
  }

  func selectedEntryMemorySummary() -> String? {
    TimelineInspectorPresenter.selectedEntryMemorySummary(timelineInspectorSnapshot())
  }

  func selectedEntrySandboxSummary() -> String? {
    TimelineInspectorPresenter.selectedEntrySandboxSummary(timelineInspectorSnapshot())
  }

  func shouldShowSelectedEntryInspector() -> Bool {
    SessionOverviewPresenter.shouldShowSelectedEntryInspector(sessionOverviewSnapshot())
  }

  func workspaceDisplayName() -> String {
    SessionOverviewPresenter.workspaceDisplayName(sessionOverviewSnapshot())
  }

  func workspacePath() -> String {
    SessionOverviewPresenter.workspacePath(sessionOverviewSnapshot())
  }

  func isPendingApproval(_ entry: TimelineEntry) -> Bool {
    guard entry.kind == .approval,
          let approvalID = entry.attributes["approvalId"]
    else {
      return false
    }

    return canRespondToApproval(approvalID: approvalID)
  }

  func approvalID(for entry: TimelineEntry) -> String? {
    entry.attributes["approvalId"]
  }

  private func timelineInspectorSnapshot() -> TimelineInspectorSnapshot {
    TimelineInspectorSnapshot(selectedEntry: selectedEntry())
  }

  private func sessionOverviewSnapshot() -> SessionOverviewSnapshot {
    let selectedThread = selectedThreadID.flatMap { threadID in
      threads.first(where: { $0.id == threadID })
    }

    return SessionOverviewSnapshot(
      selectedThread: selectedThread,
      workspace: workspace,
      selectedEntry: selectedEntry()
    )
  }
}
