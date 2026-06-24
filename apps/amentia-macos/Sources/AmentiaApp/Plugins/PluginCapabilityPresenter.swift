import SwiftUI

enum PluginCapabilityPresenter {
  static func title(_ capability: PluginCapabilitySummary) -> String {
    if capability.kind == "connector",
       let displayName = cleanMetadataValue(capability.metadata["displayName"])
    {
      return "Connection: \(displayName)"
    }

    if capability.kind == "connector_workflow",
       let displayName = cleanMetadataValue(capability.metadata["displayName"])
    {
      return "Workflow: \(displayName)"
    }

    return PluginCapabilityDisplay.surface(capability.kind)
  }

  static func reviewSummary(_ capability: PluginCapabilitySummary) -> String? {
    switch capability.kind {
    case "connector":
      return connectorReviewSummary(capability)
    case "skill":
      return skillReviewSummary(capability)
    case "mcp_server":
      return mcpServerReviewSummary(capability)
    case "connector_workflow":
      return workflowReviewSummary(capability)
    case "tool":
      return toolReviewSummary(capability)
    case "command", "hook":
      return definitionReviewSummary(capability)
    default:
      return nil
    }
  }

  static func diagnosticSummary(_ capability: PluginCapabilitySummary) -> String? {
    if let serverStatus = capability.metadata["serverStatus"] {
      return "MCP server: \(displayStatus(serverStatus))"
    }
    if let definitionStatus = capability.metadata["definitionStatus"] {
      return "\(title(capability)) definition: \(displayStatus(definitionStatus))"
    }
    return nil
  }

  static func diagnosticDetail(_ capability: PluginCapabilitySummary) -> String? {
    if capability.metadata["serverError"] != nil {
      return "Add the missing MCP command in plugin setup."
    }
    if capability.metadata["definitionError"] != nil {
      return "Review this capability definition in plugin setup."
    }
    return nil
  }

  static func diagnosticColor(_ capability: PluginCapabilitySummary) -> Color {
    switch capability.metadata["serverStatus"] ?? capability.metadata["definitionStatus"] {
    case "ready":
      return .secondary
    case nil:
      return .secondary
    default:
      return .orange
    }
  }

  private static func connectorReviewSummary(_ capability: PluginCapabilitySummary) -> String? {
    var parts: [String] = []
    if let service = cleanMetadataValue(capability.metadata["service"]) {
      parts.append("Service: \(PluginStatusDisplay.serviceName(service))")
    }
    if let authRequired = capability.metadata["authRequired"] {
      parts.append(authRequired == "true" ? "authorization required" : "no authorization")
    }
    if let authType = cleanMetadataValue(capability.metadata["authType"]) {
      parts.append("auth: \(PluginStatusDisplay.authTypeName(authType))")
    }
    if let access = PluginStatusDisplay.accessSummary(capability.metadata["authScopes"]) {
      parts.append("access: \(access)")
    }
    if let credentialStore = cleanMetadataValue(capability.metadata["credentialStore"]) {
      parts.append("token: \(PluginStatusDisplay.credentialStoreName(credentialStore))")
    }
    return parts.isEmpty ? nil : parts.joined(separator: " | ")
  }

  private static func skillReviewSummary(_ capability: PluginCapabilitySummary) -> String? {
    guard let description = cleanMetadataValue(capability.metadata["description"]) else {
      return nil
    }

    return "Guidance: \(description)"
  }

  private static func mcpServerReviewSummary(_ capability: PluginCapabilitySummary) -> String? {
    let transport: String
    if let metadataTransport = cleanMetadataValue(capability.metadata["transport"]) {
      transport = displayTransport(metadataTransport)
    } else {
      transport = "local"
    }
    let commandState = capability.metadata["command"] == nil
      ? "needs a local command"
      : "local command configured"
    return "MCP: \(transport) server, \(commandState)."
  }

  private static func definitionReviewSummary(_ capability: PluginCapabilitySummary) -> String? {
    guard let status = capability.metadata["definitionStatus"] else {
      return nil
    }

    switch status {
    case "ready":
      return "Setup: definition ready."
    case "missing":
      return "Setup: definition missing."
    case "invalid":
      return "Setup: definition needs review."
    default:
      return "Setup: \(displayStatus(status))."
    }
  }

  private static func workflowReviewSummary(_ capability: PluginCapabilitySummary) -> String? {
    var parts: [String] = []
    if let service = cleanMetadataValue(capability.metadata["service"]) {
      parts.append("service: \(PluginStatusDisplay.serviceName(service))")
    }
    if let action = cleanMetadataValue(capability.metadata["action"]) {
      parts.append("action: \(displayAction(action))")
    }
    if let maxSteps = cleanMetadataValue(capability.metadata["maxAgentSteps"]) {
      parts.append("limit: up to \(maxSteps) steps")
    }
    if parts.isEmpty {
      return "Workflow: local connector workflow."
    }
    return "Workflow: \(parts.joined(separator: " | "))"
  }

  private static func toolReviewSummary(_ capability: PluginCapabilitySummary) -> String? {
    if let description = cleanMetadataValue(capability.metadata["description"]) {
      return "Tool: \(description)"
    }
    return "Tool: bounded local capability."
  }

  private static func displayStatus(_ status: String) -> String {
    switch status {
    case "missingCommand":
      return "missing command"
    case "unsupportedTransport":
      return "unsupported transport"
    default:
      return status
    }
  }

  private static func displayTransport(_ value: String) -> String {
    switch value {
    case "stdio":
      return "local stdio"
    default:
      return value.replacingOccurrences(of: "_", with: " ")
    }
  }

  private static func displayAction(_ value: String) -> String {
    value
      .replacingOccurrences(of: "_", with: " ")
      .replacingOccurrences(of: "-", with: " ")
  }

  private static func cleanMetadataValue(_ value: String?) -> String? {
    guard let value = value?.trimmingCharacters(in: .whitespacesAndNewlines),
          !value.isEmpty,
          !value.contains("/"),
          !value.contains("\\")
    else {
      return nil
    }

    return value
  }
}
