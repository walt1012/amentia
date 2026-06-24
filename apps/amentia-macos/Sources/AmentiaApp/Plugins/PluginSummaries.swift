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
  let workflows: [PluginConnectorWorkflowSummary]
  let authStatus: String
  let credentialPresent: Bool
  let credentialSecretPresent: Bool
  let credentialProvider: String?
  let credentialHandle: String?
  let credentialLabel: String?
  let authorizedAt: Int?
  let credentialUpdatedAt: Int?
}

struct PluginConnectorWorkflowSummary: Hashable {
  let workflowID: String
  let displayName: String
  let connectorID: String
  let service: String
  let action: String
  let maxAgentSteps: Int?
  let stages: [String]
  let statuses: [String]
  let commandIDs: [String]

  var commandCoverageLabel: String {
    commandIDs.isEmpty
      ? "no commands"
      : "\(commandIDs.count) command\(commandIDs.count == 1 ? "" : "s")"
  }

  var stepBudgetLabel: String? {
    maxAgentSteps.map { "up to \($0) steps" }
  }

  var workflowLabel: String {
    ([displayName, action, commandCoverageLabel] + [stepBudgetLabel].compactMap { $0 })
      .joined(separator: " / ")
  }
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
  let runRepairHint: String?
  let declaredConnectorIds: [String]
  let requiredConnectorIds: [String]
  let approvalRequired: Bool
  let approvalReason: String?
}

struct PluginCommandExecutionSummary: Hashable {
  let kind: String
  let driver: String
  let entrypoint: String?
  let workflowID: String?
  let workflow: PluginCommandWorkflowSummary?
  let input: PluginCommandEnvelopeSummary?
  let output: PluginCommandEnvelopeSummary?
  let supported: Bool
}

struct PluginCommandWorkflowSummary: Hashable {
  let workflowID: String
  let displayName: String
  let connectorID: String
  let service: String
  let action: String
  let maxAgentSteps: Int?
  let stages: [String]
  let statuses: [String]
  let commandIDs: [String]

  var commandCoverageLabel: String {
    commandIDs.isEmpty
      ? "no commands"
      : "\(commandIDs.count) command\(commandIDs.count == 1 ? "" : "s")"
  }

  var stepBudgetLabel: String? {
    maxAgentSteps.map { "up to \($0) steps" }
  }

  var workflowLabel: String {
    ([displayName, action, commandCoverageLabel] + [stepBudgetLabel].compactMap { $0 })
      .joined(separator: " / ")
  }
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

  var visibleConnectorIds: [String] {
    requiredConnectorIds.isEmpty ? declaredConnectorIds : requiredConnectorIds
  }

  var unsupportedRequiredInputFieldNames: [String] {
    requiredInputFields
      .filter { !$0.isSupportedByAmentiaCommandRun }
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

  var isSupportedByAmentiaCommandRun: Bool {
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
  let status: String
  let runBlocker: String?
  let runRepairHint: String?
  let memorySummary: String?
}

struct PluginSkillSummary: Identifiable, Hashable {
  let id: String
  let description: String
  let pluginID: String
  let pluginDisplayName: String
  let permissions: [String]
  let sourcePath: String
  let status: String
  let preview: String?
  let contentBytes: Int
  let runBlocker: String?
  let runRepairHint: String?
}
