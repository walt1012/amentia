import Foundation

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
  let execution: PluginCommandExecutionSummary?
  let executionKind: String?
  let memorySummary: String?
}

struct PluginCommandExecutionSummary: Hashable {
  let kind: String
  let driver: String
  let entrypoint: String?
  let supported: Bool
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
