import Foundation

extension TimelineEventPresenter {
  static func firstRequestReady() -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "First Request Ready",
      body:
        "Runtime, local model, workspace, and thread are ready. Send one short local request to finish first-use setup.",
      attributes: [
        "setup": "first-request"
      ]
    )
  }

  static func runtimeDisconnected(detail: String) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Runtime Disconnected",
      body: "\(detail) Use Relaunch Runtime to recover the local session.",
      attributes: [
        "recovery": "relaunch-runtime"
      ]
    )
  }

  static func runtimeLaunchFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Runtime Launch Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }
}
