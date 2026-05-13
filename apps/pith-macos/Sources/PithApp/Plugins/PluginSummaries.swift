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
  let authStatus: String
  let credentialPresent: Bool
  let credentialSecretPresent: Bool
  let credentialProvider: String?
  let credentialHandle: String?
  let credentialLabel: String?
  let authorizedAt: Int?
  let credentialUpdatedAt: Int?
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
  let runStatus: String
  let runBlocker: String?
  let declaredConnectorIds: [String]
  let requiredConnectorIds: [String]
  let approvalRequired: Bool
  let approvalReason: String?
}

struct PluginCommandExecutionSummary: Hashable {
  let kind: String
  let driver: String
  let entrypoint: String?
  let input: PluginCommandEnvelopeSummary?
  let output: PluginCommandEnvelopeSummary?
  let supported: Bool
}

struct PluginCommandEnvelopeSummary: Hashable {
  let envelope: String
  let fields: [PluginCommandEnvelopeFieldSummary]
}

struct PluginCommandEnvelopeFieldSummary: Hashable {
  let name: String
  let kind: String
  let required: Bool
  let description: String?
}

extension PluginCommandSummary {
  var acceptsPlainInput: Bool {
    inputField?.isPlainTextInput == true
  }

  var requiresPlainInput: Bool {
    inputField.map { $0.required && $0.isPlainTextInput } ?? false
  }

  var requiresWorkspaceInput: Bool {
    requiredInputFields.contains { $0.name == "workspace" }
  }

  var requiresConnectorInput: Bool {
    requiredInputFields.contains { $0.name == "connectors" }
  }

  var requiredInputFieldNames: [String] {
    requiredInputFields.map(\.name)
  }

  var unsupportedRequiredInputFieldNames: [String] {
    requiredInputFields
      .filter { !$0.isSupportedByPithCommandRun }
      .map(\.name)
  }

  private var inputField: PluginCommandEnvelopeFieldSummary? {
    execution?.input?.fields.first { $0.name == "input" }
  }

  private var requiredInputFields: [PluginCommandEnvelopeFieldSummary] {
    execution?.input?.fields.filter(\.required) ?? []
  }
}

private extension PluginCommandEnvelopeFieldSummary {
  var isPlainTextInput: Bool {
    let normalizedKind = kind.lowercased()
    return normalizedKind == "text" || normalizedKind == "string"
  }

  var isSupportedByPithCommandRun: Bool {
    switch name {
    case "threadId", "commandId", "envelope", "workspace", "connectors":
      return true
    case "input":
      return isPlainTextInput
    default:
      return false
    }
  }
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
