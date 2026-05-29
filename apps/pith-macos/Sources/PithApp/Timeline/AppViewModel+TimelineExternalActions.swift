import AppKit
import Foundation

@MainActor
extension AppViewModel {
  func timelineExternalAction(from entry: TimelineEntry) -> TimelineExternalActionSummary? {
    TimelineExternalActionPresenter.primaryAction(
      attributes: entry.attributes,
      workspaceRoot: workspace?.rootPath
    )
  }

  func timelineProofSummary(from entry: TimelineEntry) -> TimelineProofSummary? {
    TimelineExternalActionPresenter.proofSummary(attributes: entry.attributes)
  }

  func openTimelineExternalAction(from entry: TimelineEntry) {
    guard let action = timelineExternalAction(from: entry) else {
      runtimeDetail = "External timeline action is unavailable."
      return
    }

    if action.url.isFileURL {
      runtimeDetail = FileRevealService.revealFilePath(
        action.url.path,
        successDetail: "Revealed timeline source: \(action.title)."
      )
      return
    }

    if NSWorkspace.shared.open(action.url) {
      runtimeDetail = "Opened external proof: \(action.title)."
    } else {
      runtimeDetail = "Could not open external proof: \(action.url.absoluteString)."
    }
  }

  func copyTimelineExternalActionURL(from entry: TimelineEntry) {
    guard let action = timelineExternalAction(from: entry) else {
      runtimeDetail = "External timeline action is unavailable."
      return
    }

    let pasteboard = NSPasteboard.general
    pasteboard.clearContents()
    pasteboard.setString(action.copyValue ?? action.url.absoluteString, forType: .string)
    runtimeDetail = "Copied timeline action target: \(action.title)."
  }
}
