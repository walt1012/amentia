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

struct PluginConnectorCredentialParams: Codable {
  let connectorId: String
  let credentialLabel: String?
  let credentialSecret: String?

  init(
    connectorId: String,
    credentialLabel: String? = nil,
    credentialSecret: String? = nil
  ) {
    self.connectorId = connectorId
    self.credentialLabel = credentialLabel
    self.credentialSecret = credentialSecret
  }
}

struct PluginConnectorCredentialResult: Codable {
  let connector: RuntimePluginConnectorPayload
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
  let execution: RuntimePluginCommandExecutionPayload?
  let executionKind: String?
  let memorySummary: String?
  let runStatus: String
  let runBlocker: String?
  let requiredConnectorIds: [String]
}

struct RuntimePluginCommandExecutionPayload: Codable {
  let kind: String
  let driver: String
  let entrypoint: String?
  let input: RuntimePluginCommandEnvelopePayload?
  let output: RuntimePluginCommandEnvelopePayload?
  let supported: Bool
}

struct RuntimePluginCommandEnvelopePayload: Codable {
  let envelope: String
  let fields: [RuntimePluginCommandEnvelopeFieldPayload]
}

struct RuntimePluginCommandEnvelopeFieldPayload: Codable {
  let name: String
  let kind: String
  let required: Bool
  let description: String?
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
  let authStatus: String
  let credentialPresent: Bool
  let credentialSecretPresent: Bool
  let credentialProvider: String?
  let credentialHandle: String?
  let credentialLabel: String?
  let authorizedAt: Int?
  let credentialUpdatedAt: Int?
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

extension RuntimeBridge {
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
    let execution: RuntimePluginCommandExecution?
    let executionKind: String?
    let memorySummary: String?
    let runStatus: String
    let runBlocker: String?
    let requiredConnectorIds: [String]
  }

  struct RuntimePluginCommandExecution {
    let kind: String
    let driver: String
    let entrypoint: String?
    let input: RuntimePluginCommandEnvelope?
    let output: RuntimePluginCommandEnvelope?
    let supported: Bool
  }

  struct RuntimePluginCommandEnvelope {
    let envelope: String
    let fields: [RuntimePluginCommandEnvelopeField]
  }

  struct RuntimePluginCommandEnvelopeField {
    let name: String
    let kind: String
    let required: Bool
    let description: String?
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
    let authStatus: String
    let credentialPresent: Bool
    let credentialSecretPresent: Bool
    let credentialProvider: String?
    let credentialHandle: String?
    let credentialLabel: String?
    let authorizedAt: Int?
    let credentialUpdatedAt: Int?
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
}
