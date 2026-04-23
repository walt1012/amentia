import Foundation

struct JSONRPCRequest<Params: Encodable>: Encodable {
  let id: Int
  let method: String
  let params: Params
}

struct JSONRPCResponse<ResultType: Decodable>: Decodable {
  let id: Int?
  let result: ResultType?
  let error: JSONRPCError?
}

struct JSONRPCError: Decodable {
  let code: Int
  let message: String
}

struct ClientInfo: Codable {
  let name: String
  let version: String
}

struct InitializeParams: Codable {
  let clientInfo: ClientInfo
}

struct ServerInfo: Codable {
  let name: String
  let version: String
}

struct ServerCapabilities: Codable {
  let supportsThreads: Bool
  let supportsTools: Bool
  let supportsPlugins: Bool
}

struct InitializeResult: Codable {
  let serverInfo: ServerInfo
  let protocolVersion: String
  let capabilities: ServerCapabilities
}

struct ThreadListResult: Codable {
  let threads: [RuntimeThreadPayload]
}

struct RuntimeWorkspacePayload: Codable {
  let rootPath: String
  let displayName: String
}

struct WorkspaceOpenParams: Codable {
  let path: String
}

struct WorkspaceOpenResult: Codable {
  let workspace: RuntimeWorkspacePayload
  let threadCount: Int
}

struct RuntimeThreadPayload: Codable {
  let id: String
  let title: String
  let status: String
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

struct ThreadReadResult: Codable {
  let thread: RuntimeThreadPayload
  let items: [RuntimeTimelineItem]
  let pendingApprovals: [RuntimeApprovalPayload]
  let activeTurnId: String?
}

struct TurnStartParams: Codable {
  let threadId: String
  let message: String
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

struct TurnCancelResult: Codable {
  let turnId: String
  let threadId: String
  let items: [RuntimeTimelineItem]
  let activeTurnId: String?
}

struct OptionalRequestParams: Encodable {
  static let none = OptionalRequestParams()
}
