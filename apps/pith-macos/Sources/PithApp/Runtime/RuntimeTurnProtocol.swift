import Foundation

struct ThreadListResult: Codable {
  let threads: [RuntimeThreadPayload]
}

struct RuntimeThreadPayload: Codable {
  let id: String
  let title: String
  let status: String
  let workspace: RuntimeWorkspacePayload?
}

struct ThreadStartParams: Codable {
  let title: String
}

struct ThreadStartResult: Codable {
  let thread: RuntimeThreadPayload
}

struct ThreadReadParams: Codable {
  let threadId: String
}

struct ThreadDeleteParams: Codable {
  let threadId: String
}

struct ThreadDeleteResult: Codable {
  let threadId: String
  let deleted: Bool
  let threads: [RuntimeThreadPayload]
}

struct ThreadReadResult: Codable {
  let thread: RuntimeThreadPayload
  let items: [RuntimeTimelineItem]
  let pendingApprovals: [RuntimeApprovalPayload]
  let activeTurnId: String?
}

struct ThreadUpdatedNotificationParams: Codable {
  let thread: RuntimeThreadPayload
  let items: [RuntimeTimelineItem]
  let pendingApprovals: [RuntimeApprovalPayload]
  let activeTurnId: String?
}

struct TurnStartParams: Codable {
  let threadId: String
  let message: String
  let localExecutionSafetyMode: String?
}

struct RuntimeTimelineItem: Codable {
  let kind: String
  let title: String
  let content: String
  let attributes: [String: String]?
}

struct RuntimeApprovalPayload: Codable {
  let id: String
  let threadId: String
  let action: String
  let title: String
  let relativePath: String
}

struct ApprovalRespondParams: Codable {
  let approvalId: String
  let decision: String
}

struct TurnStartResult: Codable {
  let turnId: String
  let threadId: String
  let items: [RuntimeTimelineItem]
  let pendingApprovals: [RuntimeApprovalPayload]
  let activeTurnId: String?
}

struct ApprovalRespondResult: Codable {
  let approvalId: String
  let threadId: String
  let items: [RuntimeTimelineItem]
  let pendingApprovals: [RuntimeApprovalPayload]
}

struct TurnCancelParams: Codable {
  let turnId: String
}

struct TurnCancelRunningParams: Codable {
  let threadId: String
}

struct TurnCancelResult: Codable {
  let turnId: String?
  let threadId: String
  let items: [RuntimeTimelineItem]
  let activeTurnId: String?
}

extension RuntimeBridge {
  struct RuntimeThreadSummary {
    let id: String
    let title: String
    let status: String
    let workspaceRootPath: String?
    let workspaceDisplayName: String?
  }

  struct RuntimeTurnResult {
    let turnID: String
    let threadID: String
    let items: [RuntimeTimelineItemResult]
    let pendingApprovals: [RuntimeApproval]
    let activeTurnID: String?
  }

  struct RuntimeThreadState {
    let id: String
    let title: String
    let status: String
    let items: [RuntimeTimelineItemResult]
    let pendingApprovals: [RuntimeApproval]
    let activeTurnID: String?
  }

  struct RuntimeTimelineItemResult {
    let kind: String
    let title: String
    let content: String
    let attributes: [String: String]
  }

  struct RuntimeApproval {
    let id: String
    let threadID: String
    let action: String
    let title: String
    let relativePath: String
  }

  struct RuntimeApprovalResponse {
    let approvalID: String
    let threadID: String
    let items: [RuntimeTimelineItemResult]
    let pendingApprovals: [RuntimeApproval]
  }

  struct RuntimeCancellationResult {
    let turnID: String?
    let threadID: String
    let items: [RuntimeTimelineItemResult]
    let activeTurnID: String?
  }
}
