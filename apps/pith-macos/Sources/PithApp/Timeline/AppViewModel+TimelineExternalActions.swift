import AppKit
import Foundation

@MainActor
extension AppViewModel {
  func timelineExternalAction(from entry: TimelineEntry) -> TimelineExternalActionSummary? {
    TimelineExternalActionPresenter.primaryAction(attributes: entry.attributes)
  }

  func openTimelineExternalAction(from entry: TimelineEntry) {
    guard let action = timelineExternalAction(from: entry) else {
      runtimeDetail = "External timeline action is unavailable."
      return
    }

    if NSWorkspace.shared.open(action.url) {
      runtimeDetail = "Opened external proof: \(action.title)."
    } else {
      runtimeDetail = "Could not open external proof: \(action.url.absoluteString)."
    }
  }
}
