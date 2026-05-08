import Foundation

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

struct PluginConnectorRegistryResult: Codable {
  let connectors: [RuntimePluginConnectorPayload]
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

struct RuntimePluginConnectorPayload: Codable {
  let connectorId: String
  let displayName: String
  let service: String
  let pluginId: String
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
