import Foundation

enum TimelineEventPresenter {
  static let generatingLocalResponseDetail = "Generating local response..."
  static let pendingTurnCancelledDetail = "Local execution request cancelled."
  static let runningPluginCommandDetail = "Running local plugin command..."
  static let pluginCommandNeedsExecutionContractDetail =
    "Plugin command needs an execution contract before it can run."
  static let pendingPluginCommandCancelledDetail = "Local plugin command cancelled."
  static let cancellingTurnDetail = "Cancelling local execution..."

  static let cancelledResponsePreview = "Cancelled response"
  static let cancellingResponsePreview = "Cancelling response"
  static let cancelledPluginCommandPreview = "Cancelled plugin command"

  static func turnPreview(turnID: String, activeTurnID: String?) -> String {
    activeTurnID == nil ? "\(turnID) ready" : "Streaming response"
  }

  static func threadCreationFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Thread Creation Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func threadCreated(_ thread: ThreadSummary) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Thread Created",
      body: "Created \(thread.title) in the local runtime.",
      attributes: [:]
    )
  }

  static func pendingTurnCancelled() -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Execution Cancelled",
      body: "The pending local execution request was cancelled before it finished.",
      attributes: [:]
    )
  }

  static func turnFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Turn Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func approvalResponseFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Approval Response Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func turnCancelFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Execution Cancel Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func threadLoadFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Thread Load Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

}
