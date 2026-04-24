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

struct JSONRPCAnyResponse: Decodable {
  let id: Int?
}

struct JSONRPCNotificationEnvelope: Decodable {
  let method: String
}

struct JSONRPCNotification<Params: Decodable>: Decodable {
  let method: String
  let params: Params
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
  let supportsMemory: Bool
  let supportsThreads: Bool
  let supportsTools: Bool
  let supportsPlugins: Bool
}

struct InitializeResult: Codable {
  let serverInfo: ServerInfo
  let protocolVersion: String
  let capabilities: ServerCapabilities
}

struct ModelHealthResult: Codable {
  let packId: String
  let displayName: String
  let backend: String
  let status: String
  let detail: String
  let source: String
  let binaryPath: String?
  let modelPath: String?
  let manifestPath: String?
  let metrics: [String: String]
}

struct ModelBootstrapResult: Codable {
  let manifestPath: String
  let readmePath: String?
  let copiedFiles: [String]
}

struct MemoryStatusResult: Codable {
  let noteCount: Int
  let latestTitle: String?
  let summary: String
}

struct MemoryListResult: Codable {
  let notes: [RuntimeMemoryNotePayload]
}

struct MemoryCreateParams: Codable {
  let title: String
  let body: String
}

struct MemoryCreateResult: Codable {
  let note: RuntimeMemoryNotePayload
}

struct RuntimeMemoryNotePayload: Codable {
  let id: String
  let title: String
  let body: String
  let scope: String
  let source: String
  let createdAt: Int
  let tags: [String]
}

struct PluginListResult: Codable {
  let plugins: [RuntimePluginPayload]
}

struct PluginInstallParams: Codable {
  let sourcePath: String
}

struct PluginInstallResult: Codable {
  let plugin: RuntimePluginPayload
}

struct PluginRemoveParams: Codable {
  let manifestPath: String
}

struct PluginRemoveResult: Codable {
  let pluginId: String
  let displayName: String
  let removedPath: String
}

struct PluginCapabilityRegistryResult: Codable {
  let capabilities: [RuntimePluginCapabilityPayload]
  let summary: RuntimePluginCapabilityRegistrySummaryPayload
}

struct PluginCommandRegistryResult: Codable {
  let commands: [RuntimePluginCommandPayload]
}

struct PluginHookRegistryResult: Codable {
  let hooks: [RuntimePluginHookPayload]
}

struct RuntimePluginCapabilityRegistrySummaryPayload: Codable {
  let enabledPluginCount: Int
  let totalCapabilityCount: Int
  let capabilityCountsByKind: [String: Int]
}

struct RuntimePluginCapabilityPayload: Codable {
  let capabilityId: String
  let kind: String
  let identifier: String
  let pluginId: String
  let pluginDisplayName: String
  let permissions: [String]
  let manifestPath: String
  let metadata: [String: String]?
}

struct RuntimePluginCommandPayload: Codable {
  let commandId: String
  let title: String
  let description: String
  let pluginId: String
  let pluginDisplayName: String
  let permissions: [String]
  let sourcePath: String
  let executionKind: String?
  let memorySummary: String?
}

struct RuntimePluginHookPayload: Codable {
  let hookId: String
  let title: String
  let description: String
  let event: String
  let pluginId: String
  let pluginDisplayName: String
  let permissions: [String]
  let sourcePath: String
  let memorySummary: String?
}

struct RuntimePluginPayload: Codable {
  let id: String
  let name: String
  let version: String
  let displayName: String
  let status: String
  let description: String
  let authorName: String?
  let enabled: Bool
  let defaultEnabled: Bool
  let capabilities: [String]
  let permissions: [String]
  let manifestPath: String
  let provenance: String
  let validationError: String?
  let validationHint: String?
}

struct PluginSetEnabledParams: Codable {
  let pluginId: String
  let enabled: Bool
}

struct PluginSetEnabledResult: Codable {
  let plugin: RuntimePluginPayload
}

struct PluginCommandRunParams: Codable {
  let threadId: String
  let commandId: String
  let input: String?
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

struct WorkspaceCurrentResult: Codable {
  let workspace: RuntimeWorkspacePayload?
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

struct ThreadUpdatedNotificationParams: Codable {
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
