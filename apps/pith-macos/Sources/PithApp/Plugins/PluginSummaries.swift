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

struct PluginSurfaceSummary: Hashable {
  let commandCount: Int
  let connectorCount: Int
  let mcpServerCount: Int
  let hookCount: Int
  let permissionCount: Int

  var preferredSection: PluginManagerSection {
    if connectorCount > 0 || mcpServerCount > 0 {
      return .connectors
    }
    if commandCount > 0 {
      return .commands
    }
    if hookCount > 0 {
      return .hooks
    }
    if permissionCount > 0 {
      return .access
    }
    return .catalog
  }

  var summary: String {
    var parts = [
      "\(commandCount) command\(commandCount == 1 ? "" : "s")",
      "\(connectorCount) connector\(connectorCount == 1 ? "" : "s")",
      "\(hookCount) hook\(hookCount == 1 ? "" : "s")",
    ]
    if mcpServerCount > 0 {
      parts.append("\(mcpServerCount) MCP server\(mcpServerCount == 1 ? "" : "s")")
    }
    parts.append("\(permissionCount) permission\(permissionCount == 1 ? "" : "s")")
    return parts.joined(separator: " | ")
  }
}

enum PluginSurfaceClassifier {
  static func summary(
    capabilities: [String],
    permissions: [String]
  ) -> PluginSurfaceSummary {
    PluginSurfaceSummary(
      commandCount: count(capabilities, kind: "command"),
      connectorCount: count(capabilities, kind: "connector"),
      mcpServerCount: count(capabilities, kind: "mcp_server"),
      hookCount: count(capabilities, kind: "hook"),
      permissionCount: permissions.count
    )
  }

  static func preferredSection(
    capabilities: [String],
    permissions: [String]
  ) -> PluginManagerSection {
    summary(capabilities: capabilities, permissions: permissions).preferredSection
  }

  private static func count(_ capabilities: [String], kind: String) -> Int {
    capabilities.filter { $0.hasPrefix("\(kind):") }.count
  }
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
  var explicitTurnRoute: String {
    "/plugin \(id)"
  }

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
  let status: String
  let runBlocker: String?
  let runRepairHint: String?
  let memorySummary: String?
}
