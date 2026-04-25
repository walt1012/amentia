import Foundation

struct ThreadSummary: Identifiable, Hashable {
  let id: String
  var title: String
  var preview: String
  var workspaceRootPath: String?
  var workspaceDisplayName: String?
}

struct WorkspaceSummary: Hashable {
  let rootPath: String
  let displayName: String
}

struct WorkspaceSearchMatchSummary: Identifiable, Hashable {
  let id: String
  let relativePath: String
  let lineNumber: Int
  let line: String
}

struct ModelHealthSummary: Hashable {
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

struct LocalModelSummary: Identifiable, Hashable {
  let id: String
  let displayName: String
  let description: String
  let fileName: String
  let downloadURL: String
  let homepage: String
  let sizeBytes: Int64
  let contextSize: Int
  let maxOutputTokens: Int
  let license: String
  let tags: [String]
  let installPath: String
  let downloaded: Bool
  let active: Bool
  let localSizeBytes: Int64?
}

struct MemoryStatusSummary: Hashable {
  let noteCount: Int
  let latestTitle: String?
  let summary: String
}

struct MemoryNoteSummary: Identifiable, Hashable {
  let id: String
  let title: String
  let body: String
  let scope: String
  let source: String
  let createdAt: Int
  let tags: [String]
}

struct PluginSummary: Identifiable, Hashable {
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

struct PluginCapabilityRegistrySummary: Hashable {
  let enabledPluginCount: Int
  let totalCapabilityCount: Int
  let capabilityCountsByKind: [String: Int]
}

struct PluginCapabilitySummary: Identifiable, Hashable {
  let id: String
  let kind: String
  let identifier: String
  let pluginID: String
  let pluginDisplayName: String
  let permissions: [String]
  let manifestPath: String
  let metadata: [String: String]
}

struct PluginConnectorSummary: Identifiable, Hashable {
  let id: String
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

struct PluginCommandSummary: Identifiable, Hashable {
  let id: String
  let title: String
  let description: String
  let pluginID: String
  let pluginDisplayName: String
  let permissions: [String]
  let sourcePath: String
  let executionKind: String?
  let memorySummary: String?
}

struct PluginHookSummary: Identifiable, Hashable {
  let id: String
  let title: String
  let description: String
  let event: String
  let pluginID: String
  let pluginDisplayName: String
  let permissions: [String]
  let sourcePath: String
  let memorySummary: String?
}

struct TimelineEntry: Identifiable, Hashable {
  enum Kind: String {
    case userMessage
    case assistantMessage
    case system
    case plan
    case tool
    case diff
    case approval
    case warning
  }

  let id: String
  let kind: Kind
  let title: String
  let body: String
  let attributes: [String: String]
}

enum DiffLineKind: String, Hashable {
  case addition
  case deletion
  case hunk
  case metadata
  case context
}

enum StatusTone: String, Hashable {
  case neutral
  case ready
  case active
  case warning
  case danger
}

struct ReadinessStepSummary: Identifiable, Hashable {
  let id: String
  let label: String
  let detail: String
  let tone: StatusTone
}

struct ComposerSuggestionSummary: Identifiable, Hashable {
  let id: String
  let title: String
  let message: String
}

struct DiffLineSummary: Identifiable, Hashable {
  let id: String
  let lineNumber: Int
  let text: String
  let kind: DiffLineKind
}
