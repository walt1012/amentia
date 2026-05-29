import Foundation

extension RuntimeBridge {
  func listThreads() async throws -> [RuntimeThreadSummary] {
    let response: JSONRPCResponse<ThreadListResult> = try await sendRequest(
      method: "thread/list",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return result.threads.map {
      RuntimeThreadSummary(
        id: $0.id,
        title: $0.title,
        status: $0.status,
        workspaceRootPath: $0.workspace?.rootPath,
        workspaceDisplayName: $0.workspace?.displayName
      )
    }
  }

  func startThread(title: String) async throws -> ThreadSummary {
    let response: JSONRPCResponse<ThreadStartResult> = try await sendRequest(
      method: "thread/start",
      params: ThreadStartParams(title: title)
    )
    let result = try responseResult(from: response)

    return ThreadSummary(
      id: result.thread.id,
      title: result.thread.title,
      preview: result.thread.status,
      workspaceRootPath: result.thread.workspace?.rootPath,
      workspaceDisplayName: result.thread.workspace?.displayName
    )
  }

  func startTurn(
    threadID: String,
    message: String,
    localExecutionSafetyMode: String?
  ) async throws -> RuntimeTurnResult {
    let response: JSONRPCResponse<TurnStartResult> = try await sendRequest(
      method: "turn/start",
      params: TurnStartParams(
        threadId: threadID,
        message: message,
        localExecutionSafetyMode: localExecutionSafetyMode
      )
    )
    let result = try responseResult(from: response)

    return RuntimeTurnResult(
      turnID: result.turnId,
      threadID: result.threadId,
      items: result.items.map(RuntimeBridgePayloadMapper.timelineItem(from:)),
      pendingApprovals: result.pendingApprovals.map(RuntimeBridgePayloadMapper.approval(from:)),
      activeTurnID: result.activeTurnId
    )
  }

  func readThread(threadID: String) async throws -> RuntimeThreadState {
    let response: JSONRPCResponse<ThreadReadResult> = try await sendRequest(
      method: "thread/read",
      params: ThreadReadParams(threadId: threadID)
    )
    let result = try responseResult(from: response)

    return RuntimeBridgePayloadMapper.threadState(
      id: result.thread.id,
      title: result.thread.title,
      status: result.thread.status,
      items: result.items,
      pendingApprovals: result.pendingApprovals,
      activeTurnID: result.activeTurnId
    )
  }

  func respondToApproval(approvalID: String, decision: String) async throws -> RuntimeApprovalResponse {
    let response: JSONRPCResponse<ApprovalRespondResult> = try await sendRequest(
      method: "approval/respond",
      params: ApprovalRespondParams(approvalId: approvalID, decision: decision)
    )
    let result = try responseResult(from: response)

    return RuntimeApprovalResponse(
      approvalID: result.approvalId,
      threadID: result.threadId,
      items: result.items.map(RuntimeBridgePayloadMapper.timelineItem(from:)),
      pendingApprovals: result.pendingApprovals.map(RuntimeBridgePayloadMapper.approval(from:))
    )
  }

  func cancelTurn(turnID: String) async throws -> RuntimeCancellationResult {
    let response: JSONRPCResponse<TurnCancelResult> = try await sendRequest(
      method: "turn/cancel",
      params: TurnCancelParams(turnId: turnID)
    )
    let result = try responseResult(from: response)

    return RuntimeCancellationResult(
      turnID: result.turnId,
      threadID: result.threadId,
      items: result.items.map(RuntimeBridgePayloadMapper.timelineItem(from:)),
      activeTurnID: result.activeTurnId
    )
  }

  func cancelRunningExecution(threadID: String) async throws -> RuntimeCancellationResult {
    let response: JSONRPCResponse<TurnCancelResult> = try await sendRequest(
      method: "turn/cancelRunning",
      params: TurnCancelRunningParams(threadId: threadID)
    )
    let result = try responseResult(from: response)

    return RuntimeCancellationResult(
      turnID: result.turnId,
      threadID: result.threadId,
      items: result.items.map(RuntimeBridgePayloadMapper.timelineItem(from:)),
      activeTurnID: result.activeTurnId
    )
  }
}
