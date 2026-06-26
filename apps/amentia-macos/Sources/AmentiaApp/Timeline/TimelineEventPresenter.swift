import Foundation

enum TimelineEventPresenter {
  static let generatingLocalResponseDetail = "Amentia is preparing a response..."
  static let pendingTurnCancelledDetail = "Request cancelled."
  static let cancellingTurnDetail = "Cancelling request..."
  static let turnFailedDetail = "Amentia could not finish the response. Your prompt is still in the composer."
  static let modelInvocationFailedDetail =
    "The local model could not finish this response. Restart Amentia or re-download the selected model."

  static let cancelledResponsePreview = "Cancelled response"
  static let cancellingResponsePreview = "Cancelling response"
  static let failedResponsePreview = "Response failed"
  static let modelInvocationFailedPreview = "Model response failed"

  static func turnPreview(turnID: String, activeTurnID: String?) -> String {
    activeTurnID == nil ? "Response ready" : "Response in progress"
  }

  static func hasModelInvocationFailure(
    _ items: [RuntimeBridge.RuntimeTimelineItemResult]
  ) -> Bool {
    hasModelInvocationFailure(in: items.map(\.attributes))
  }

  static func hasModelInvocationFailure(in itemAttributes: [[String: String]]) -> Bool {
    itemAttributes.contains { attributes in
      guard let modelStatus = attributes["modelStatus"] else {
        return false
      }
      return modelStatus != "ready" && modelStatus != "cancelled"
    }
  }

  static func threadCreationFailed(error: Error) -> TimelineEntry {
    return TimelineEntryFactory.warning(
      title: "Session Creation Failed",
      body: UserFacingFailurePresenter.threadCreationFailureBody(),
      attributes: UserFacingFailurePresenter.technicalErrorAttributes(error)
    )
  }

  static func threadCreated(_ thread: ThreadSummary) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Session Created",
      body: "Created \(thread.title) for this project.",
      attributes: [:]
    )
  }

  static func pendingTurnCancelled() -> TimelineEntry {
    return TimelineEntryFactory.warning(
      title: "Request Cancelled",
      body: "The pending request was cancelled before it finished.",
      attributes: [:]
    )
  }

  static func turnFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Request Failed",
      body: "Amentia could not finish the response. Your prompt was restored so you can try again after checking the local model.",
      attributes: [
        "failureKind": "request",
        "recovery": "retry-or-check-local-model"
      ]
    )
  }

  static func approvalResponseFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Approval Response Failed",
      body: UserFacingFailurePresenter.approvalResponseFailureBody(),
      attributes: UserFacingFailurePresenter.technicalErrorAttributes(error)
    )
  }

  static func approvalResponseFailedDetail(error: Error) -> String {
    UserFacingFailurePresenter.approvalResponseFailureDetail()
  }

  static func turnCancelFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Cancel Failed",
      body: UserFacingFailurePresenter.requestCancelFailureBody(),
      attributes: UserFacingFailurePresenter.technicalErrorAttributes(error)
    )
  }

  static func turnCancelFailedDetail(error: Error) -> String {
    UserFacingFailurePresenter.requestCancelFailureDetail()
  }

  static func threadLoadFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Session Load Failed",
      body: UserFacingFailurePresenter.threadLoadFailureBody(),
      attributes: UserFacingFailurePresenter.technicalErrorAttributes(error)
    )
  }

  static func workspaceOpenFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Project Open Failed",
      body: UserFacingFailurePresenter.workspaceOpenFailureBody(),
      attributes: UserFacingFailurePresenter.technicalErrorAttributes(error)
    )
  }

  static func workspaceOpened(_ workspace: RuntimeBridge.RuntimeWorkspace) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Project Opened",
      body: "Opened \(workspace.displayName) as the active project.",
      attributes: [
        "workspacePath": workspace.rootPath
      ]
    )
  }

  static func firstRequestReady() -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Cowork Session Ready",
      body:
        "Amentia, the local model, project, and session are ready. Send one short cowork prompt to finish first-use setup.",
      attributes: [
        "setup": "first-request"
      ]
    )
  }

  static func runtimeDisconnected(detail: String) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Amentia Disconnected",
      body: "\(detail) Use Restart Amentia to recover the session.",
      attributes: [
        "recovery": "relaunch-runtime"
      ]
    )
  }

  static func runtimeLaunchFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Amentia Launch Failed",
      body: UserFacingFailurePresenter.runtimeLaunchFailureDetail(error: error),
      attributes: UserFacingFailurePresenter.technicalErrorAttributes(error)
    )
  }

  static func localModelDownloaded(_ plan: LocalModelDownloadCompletionPlan) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Local Model Downloaded",
      body: plan.timelineBody,
      attributes: plan.attributes
    )
  }

  static func localModelEvent(
    title: String,
    body: String,
    model: LocalModelSummary,
    kind: TimelineEntry.Kind = .system,
    attributes: [String: String] = [:]
  ) -> TimelineEntry {
    var eventAttributes = attributes
    eventAttributes["modelId"] = model.id
    eventAttributes["modelPath"] = model.installPath
    eventAttributes["modelLicense"] = model.license
    return TimelineEntryFactory.entry(
      kind: kind,
      title: title,
      body: body,
      attributes: eventAttributes
    )
  }

  static func localModelActivated(_ plan: LocalModelActivationPlan) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: plan.timelineTitle,
      body: plan.timelineBody,
      attributes: plan.attributes
    )
  }

  static func localModelProbe(
    title: String,
    body: String,
    kind: TimelineEntry.Kind = .system,
    attributes: [String: String]
  ) -> TimelineEntry {
    TimelineEntryFactory.entry(
      kind: kind,
      title: title,
      body: body,
      attributes: attributes
    )
  }

  static func memoryNoteSaved(_ note: RuntimeBridge.RuntimeMemoryNote) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Memory Note Saved",
      body: "Saved project memory note \(note.title).",
      attributes: [
        "memoryNoteId": note.id,
        "memoryScope": note.scope,
        "memorySource": note.source,
      ]
    )
  }

  static func memoryNoteFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Memory Note Failed",
      body: UserFacingFailurePresenter.memoryNoteFailureBody(),
      attributes: UserFacingFailurePresenter.technicalErrorAttributes(error)
    )
  }

}
