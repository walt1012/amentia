import Foundation

enum RuntimeBridgePayloadMapper {
  static func threadState(
    id: String,
    title: String,
    status: String,
    items: [RuntimeTimelineItem],
    pendingApprovals: [RuntimeApprovalPayload],
    activeTurnID: String?
  ) -> RuntimeBridge.RuntimeThreadState {
    RuntimeBridge.RuntimeThreadState(
      id: id,
      title: title,
      status: status,
      items: items.map(timelineItem(from:)),
      pendingApprovals: pendingApprovals.map(approval(from:)),
      activeTurnID: activeTurnID
    )
  }

  static func timelineItem(
    from payload: RuntimeTimelineItem
  ) -> RuntimeBridge.RuntimeTimelineItemResult {
    RuntimeBridge.RuntimeTimelineItemResult(
      kind: payload.kind,
      title: payload.title,
      content: payload.content,
      attributes: payload.attributes ?? [:]
    )
  }

  static func approval(from payload: RuntimeApprovalPayload) -> RuntimeBridge.RuntimeApproval {
    RuntimeBridge.RuntimeApproval(
      id: payload.id,
      threadID: payload.threadId,
      action: payload.action,
      title: payload.title,
      relativePath: payload.relativePath
    )
  }
}
