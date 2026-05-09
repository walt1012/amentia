import Foundation

extension TimelineEventPresenter {
  static func workspaceOpenFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Workspace Open Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func workspaceOpened(_ workspace: RuntimeBridge.RuntimeWorkspace) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Workspace Opened",
      body: "Opened \(workspace.displayName) at \(workspace.rootPath).",
      attributes: [:]
    )
  }
}
