import Foundation

extension RuntimeBridge {
  enum ConnectionState: String {
    case disconnected
    case launching
    case ready
    case failed
  }

  struct SessionInfo {
    let serverName: String
    let serverVersion: String
  }

  struct RuntimeThreadSummary {
    let id: String
    let title: String
    let status: String
    let workspaceRootPath: String?
    let workspaceDisplayName: String?
  }

  struct RuntimeWorkspace {
    let rootPath: String
    let displayName: String
    let threadCount: Int
  }

  struct RuntimeWorkspaceSearchMatch {
    let relativePath: String
    let lineNumber: Int
    let line: String
  }

  struct RuntimeModelHealth {
    let packID: String
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

  struct RuntimeReadinessCheck {
    let id: String
    let title: String
    let status: String
    let detail: String
  }

  struct RuntimeReadiness {
    let status: String
    let summary: String
    let checks: [RuntimeReadinessCheck]
    let metrics: [String: String]
  }

  struct RuntimeModelBootstrap {
    let manifestPath: String
    let readmePath: String?
    let copiedFiles: [String]
  }

  struct RuntimeMemoryStatus {
    let noteCount: Int
    let latestTitle: String?
    let summary: String
  }

  struct RuntimeMemoryNote {
    let id: String
    let title: String
    let body: String
    let scope: String
    let source: String
    let createdAt: Int
    let tags: [String]
  }

  struct RuntimePlugin {
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

  struct RuntimePluginRemoval {
    let pluginID: String
    let displayName: String
    let removedPath: String
  }

  struct RuntimePluginCapabilityRegistry {
    let capabilities: [RuntimePluginCapability]
    let summary: RuntimePluginCapabilityRegistrySummary
  }

  struct RuntimePluginCapabilityRegistrySummary {
    let enabledPluginCount: Int
    let totalCapabilityCount: Int
    let capabilityCountsByKind: [String: Int]
  }

  struct RuntimePluginCapability {
    let capabilityID: String
    let kind: String
    let identifier: String
    let pluginID: String
    let pluginDisplayName: String
    let permissions: [String]
    let manifestPath: String
    let metadata: [String: String]
  }

  struct RuntimePluginCommand {
    let commandID: String
    let title: String
    let description: String
    let pluginID: String
    let pluginDisplayName: String
    let permissions: [String]
    let sourcePath: String
    let executionKind: String?
    let memorySummary: String?
  }

  struct RuntimePluginConnector {
    let connectorID: String
    let displayName: String
    let service: String
    let pluginID: String
    let pluginDisplayName: String
    let enabled: Bool
    let status: String
    let permissions: [String]
    let manifestPath: String
    let homepage: String?
    let authType: String?
    let authRequired: Bool
    let authScopes: [String]
    let credentialStore: String?
  }

  struct RuntimePluginHook {
    let hookID: String
    let title: String
    let description: String
    let event: String
    let pluginID: String
    let pluginDisplayName: String
    let permissions: [String]
    let sourcePath: String
    let memorySummary: String?
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

  struct RuntimeTurnCancellation {
    let turnID: String
    let threadID: String
    let items: [RuntimeTimelineItemResult]
    let activeTurnID: String?
  }

  enum RuntimeError: LocalizedError {
    case runtimePathMissing
    case runtimePipeUnavailable
    case invalidResponse
    case requestTimedOut(method: String, seconds: Int)
    case rpc(String)

    var errorDescription: String? {
      switch self {
      case .runtimePathMissing:
        return
          "The runtime binary could not be found. " +
          "Set PITH_RUNTIME_PATH to the built runtime executable."
      case .runtimePipeUnavailable:
        return "The runtime process pipes are not available."
      case .invalidResponse:
        return "The runtime returned an invalid response."
      case .requestTimedOut(let method, let seconds):
        return
          "Runtime request \(method) timed out after \(seconds) seconds. " +
          "The local runtime was stopped so it can recover cleanly."
      case .rpc(let message):
        return message
      }
    }
  }

  typealias ThreadUpdatedHandler = @Sendable (RuntimeThreadState) -> Void
  typealias ConnectionStateHandler = @Sendable (ConnectionState, String) -> Void
}
