import Foundation

struct PluginSurfaceSummary: Hashable {
  let commandCount: Int
  let connectorCount: Int
  let mcpServerCount: Int
  let skillCount: Int
  let toolCount: Int
  let hookCount: Int
  let workflowCount: Int
  let permissionCount: Int

  var preferredSection: PluginManagerSection {
    if commandCount > 0 {
      return .commands
    }
    if connectorCount > 0 {
      return .connectors
    }
    if skillCount > 0 || mcpServerCount > 0 || toolCount > 0 || workflowCount > 0 {
      return .capabilities
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
    var parts: [String] = []
    appendCount(commandCount, singular: "action", to: &parts)
    appendCount(connectorCount, singular: "connection", to: &parts)
    appendCount(skillCount, singular: "skill", to: &parts)
    if mcpServerCount > 0 {
      parts.append("\(mcpServerCount) MCP server\(mcpServerCount == 1 ? "" : "s")")
    }
    appendCount(toolCount, singular: "tool", to: &parts)
    appendCount(hookCount, singular: "check", to: &parts)
    appendCount(workflowCount, singular: "workflow", to: &parts)
    if parts.isEmpty {
      parts.append("No declared capabilities")
    }
    appendCount(permissionCount, singular: "permission", to: &parts)
    return parts.joined(separator: " | ")
  }

  private func appendCount(_ count: Int, singular: String, to parts: inout [String]) {
    guard count > 0 else {
      return
    }

    parts.append("\(count) \(singular)\(count == 1 ? "" : "s")")
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
      skillCount: count(capabilities, kind: "skill"),
      toolCount: count(capabilities, kind: "tool"),
      hookCount: count(capabilities, kind: "hook"),
      workflowCount: count(capabilities, kind: "connector_workflow"),
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

enum PluginCapabilityDisplay {
  static func summary(_ capabilities: [String]) -> String {
    let counts = capabilityCounts(capabilities)
    return summary(counts)
  }

  static func summary(_ counts: [String: Int]) -> String {
    let knownKindParts: [String] = capabilityKindOrder
      .compactMap { kind -> String? in
        guard let count = counts[kind], count > 0 else {
          return nil
        }
        return "\(count) \(label(kind, count: count))"
      }

    let knownKinds = Set(capabilityKindOrder)
    let unknownCount = counts
      .filter { !knownKinds.contains($0.key) }
      .map { $0.value }
      .reduce(0, +)
    let unknownParts = unknownCount > 0
      ? ["\(unknownCount) \(label("unknown", count: unknownCount))"]
      : []

    return (knownKindParts + unknownParts).joined(separator: " | ")
  }

  static func surface(_ kind: String) -> String {
    switch kind {
    case "command":
      return "Action"
    case "connector":
      return "Connection"
    case "skill":
      return "Skill"
    case "tool":
      return "Tool"
    case "hook":
      return "Check"
    case "mcp_server":
      return "MCP server"
    case "connector_workflow":
      return "Workflow"
    default:
      return "Capability"
    }
  }

  private static func capabilityCounts(_ capabilities: [String]) -> [String: Int] {
    capabilities.reduce(into: [String: Int]()) { result, capability in
      guard let kind = capability.split(separator: ":", maxSplits: 1).first else {
        return
      }
      result[String(kind), default: 0] += 1
    }
  }

  private static let capabilityKindOrder = [
    "command",
    "connector",
    "skill",
    "mcp_server",
    "tool",
    "hook",
    "connector_workflow",
    "agent",
    "settings",
  ]

  private static func label(_ kind: String, count: Int) -> String {
    switch kind {
    case "command":
      return count == 1 ? "action" : "actions"
    case "connector":
      return count == 1 ? "connection" : "connections"
    case "mcp_server":
      return count == 1 ? "MCP server" : "MCP servers"
    case "hook":
      return count == 1 ? "check" : "checks"
    case "connector_workflow":
      return count == 1 ? "workflow" : "workflows"
    case "skill":
      return count == 1 ? "skill" : "skills"
    case "tool":
      return count == 1 ? "tool" : "tools"
    default:
      return count == 1 ? "capability" : "capabilities"
    }
  }
}

enum PluginPermissionDisplay {
  static func summary(_ permissions: [String], empty: String = "No extra local permissions") -> String {
    guard !permissions.isEmpty else {
      return empty
    }

    return permissions
      .map(label)
      .sorted()
      .joined(separator: ", ")
  }

  static func label(_ permission: String) -> String {
    switch permission {
    case "file.read":
      return "Project read"
    case "file.write":
      return "Project write"
    case "shell.exec":
      return "Shell commands"
    case "network.outbound":
      return "Network access"
    case "workspace.background":
      return "Background project work"
    case "model.invoke":
      return "Local model use"
    case "mcp.connect":
      return "MCP access"
    default:
      return "Custom local permission"
    }
  }
}

extension PluginSummary {
  var sourceLabel: String {
    switch provenance {
    case "bundled":
      return "Built in"
    case "local":
      return "Local"
    default:
      return provenance
    }
  }

  var capabilitySummary: String {
    PluginCapabilityDisplay.summary(capabilities)
  }

  var permissionSummary: String {
    PluginPermissionDisplay.summary(permissions)
  }
}
