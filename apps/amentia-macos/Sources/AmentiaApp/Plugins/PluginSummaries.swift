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
    "prompt_pack",
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

enum PluginStatusDisplay {
  static func pluginStatus(_ status: String) -> String {
    switch status {
    case "ready":
      return "ready"
    case "disabled":
      return "disabled"
    default:
      return "needs attention"
    }
  }

  static func validationIssue(_ plugin: PluginSummary) -> String {
    "\(plugin.displayName): \(validationDetail(plugin))"
  }

  static func validationDetail(_ plugin: PluginSummary) -> String {
    let issue = plugin.validationError?.trimmingCharacters(in: .whitespacesAndNewlines)
    let summary: String
    if let issue = issue, !issue.isEmpty {
      summary = PluginValidationCopy.userFacingMessage(issue)
    } else {
      summary = "Setup needs review."
    }

    guard let hint = plugin.validationHint?.trimmingCharacters(in: .whitespacesAndNewlines),
          !hint.isEmpty
    else {
      return summary
    }

    return "\(summary) Fix: \(PluginValidationCopy.userFacingRepairHint(hint))"
  }

  static func connectionStatus(_ status: String) -> String {
    switch status {
    case "ready":
      return "ready"
    case "needsAuth":
      return "needs sign in"
    case "disabled":
      return "disabled"
    default:
      return "needs attention"
    }
  }

  static func authorizationStatus(
    _ status: String,
    authRequired: Bool,
    credentialPresent: Bool,
    credentialSecretPresent: Bool
  ) -> String {
    if status == "disabled" {
      return "disabled"
    }

    if !authRequired {
      return "ready"
    }

    if !credentialPresent || !credentialSecretPresent {
      return "needs sign in"
    }

    switch status {
    case "authorized":
      return "authorized locally"
    case "ready":
      return "authorized locally"
    case "needsAuth":
      return "needs sign in"
    case "disabled":
      return "disabled"
    default:
      return "not authorized"
    }
  }

  static func commandStatus(_ status: String) -> String {
    switch status {
    case "ready":
      return "ready"
    case "needsConnectorAuth":
      return "needs connection"
    case "unsupportedExecution":
      return "action not supported yet"
    case "missingExecution":
      return "needs setup"
    default:
      return "needs attention"
    }
  }

  static func skillStatus(_ status: String) -> String {
    switch status {
    case "ready":
      return "ready"
    case "missingSkillFile":
      return "skill file missing"
    case "invalidSkillFile":
      return "skill needs review"
    default:
      return "needs attention"
    }
  }

  static func missingConnectionSummary(count: Int) -> String {
    count == 1
      ? "A required connection is missing."
      : "\(count) required connections are missing."
  }

  static func inputFieldLabel(_ name: String) -> String {
    switch name {
    case "workspace":
      return "selected project"
    case "connectors":
      return "authorized connections"
    case "input":
      return "your input"
    default:
      return name.replacingOccurrences(of: "_", with: " ")
    }
  }

  static func authTypeName(_ authType: String?) -> String {
    let normalized = authType?
      .trimmingCharacters(in: .whitespacesAndNewlines)
      .lowercased()
      .replacingOccurrences(of: "-", with: "_")

    switch normalized {
    case "api_key", "apikey":
      return "API key"
    case "oauth2":
      return "OAuth 2.0"
    case "none":
      return "no token needed"
    case let value? where !value.isEmpty:
      return value.replacingOccurrences(of: "_", with: " ")
    default:
      return "local authorization"
    }
  }

  static func accessSummary(_ scopes: [String]) -> String? {
    let labels = scopes
      .map(scopeName)
      .filter { !$0.isEmpty }

    guard !labels.isEmpty else {
      return nil
    }

    return labels.joined(separator: ", ")
  }

  static func accessSummary(_ rawScopes: String?) -> String? {
    guard let rawScopes else {
      return nil
    }

    let scopes = rawScopes
      .split(separator: ",")
      .map { String($0).trimmingCharacters(in: .whitespacesAndNewlines) }

    return accessSummary(scopes)
  }

  static func credentialStoreName(_ store: String?) -> String {
    switch store?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
    case "none":
      return "not saved"
    case "local", "keychain":
      return "stored locally"
    default:
      return "stored locally"
    }
  }

  static func executionSummary(_ execution: PluginCommandExecutionSummary?) -> String {
    guard let execution else {
      return "needs setup"
    }

    guard execution.supported else {
      return "action not supported yet"
    }

    if let workflow = execution.workflow {
      return "workflow ready: \(workflow.workflowLabel)"
    }

    switch execution.kind {
    case "builtin":
      return "built-in Amentia action"
    case "mcp":
      return "MCP action"
    default:
      return "action ready"
    }
  }

  static func serviceName(_ service: String) -> String {
    switch service.lowercased() {
    case "github":
      return "GitHub"
    case "notion":
      return "Notion"
    default:
      return service
        .replacingOccurrences(of: "_", with: " ")
        .replacingOccurrences(of: "-", with: " ")
        .capitalized
    }
  }

  private static func scopeName(_ scope: String) -> String {
    switch scope.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
    case "read_content":
      return "read content"
    case "insert_content":
      return "create content"
    case "write_content":
      return "edit content"
    case "read_user", "read_users":
      return "read users"
    case "pages":
      return "pages"
    default:
      return scope
        .replacingOccurrences(of: "_", with: " ")
        .replacingOccurrences(of: "-", with: " ")
    }
  }
}

enum PluginValidationCopy {
  static func userFacingMessage(_ message: String) -> String {
    let normalized = message.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !normalized.isEmpty else {
      return "Setup needs review."
    }

    if containsAny(normalized, [
      "does not contain amentia-plugin.json",
      "must be a plugin directory or amentia-plugin.json file",
      "Select a plugin folder or the amentia-plugin.json manifest",
    ]) {
      return "Plugin setup file is missing."
    }

    if normalized.contains("cannot contain nested amentia-plugin.json manifests") {
      return "Plugin bundle contains another plugin bundle."
    }

    if normalized.contains("cannot contain symbolic links") {
      return "Plugin bundle must be self-contained."
    }

    if normalized.contains("is already installed") {
      return "Plugin is already installed."
    }

    if normalized.contains("Plugin manifest name") {
      return "Plugin name needs review."
    }

    if containsAny(normalized, ["correct format", "is missing", "failed to parse"]) {
      return "Plugin setup file needs review."
    }

    if containsRawSetupDetail(normalized) {
      return "Plugin setup needs review."
    }

    return normalized
  }

  static func userFacingRepairHint(_ hint: String) -> String {
    let normalized = hint.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !normalized.isEmpty else {
      return "Review the plugin setup and try again."
    }

    if containsRawSetupDetail(normalized)
      || containsAny(normalized, ["correct format", "camelCase", "displayName"])
    {
      return "Check the plugin setup file, then refresh the plugin."
    }

    return normalized
  }

  private static func containsRawSetupDetail(_ value: String) -> Bool {
    value.contains("/")
      || value.contains("\\")
      || value.contains("amentia-plugin.json")
      || value.contains("manifest")
      || value.contains("Manifest")
      || value.contains("sourcePath")
      || value.contains("definitionPath")
  }

  private static func containsAny(_ value: String, _ needles: [String]) -> Bool {
    needles.contains { value.contains($0) }
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

enum PluginHookDisplay {
  static func disableButtonTitle(_ hook: PluginHookSummary) -> String {
    "Disable \(hook.pluginDisplayName)"
  }

  static func disableDetail(_ hook: PluginHookSummary) -> String {
    "Stops \(hook.pluginDisplayName) checks from running during future activity."
  }
}

enum PluginSkillDisplay {
  static func disableButtonTitle(_ skill: PluginSkillSummary) -> String {
    "Disable \(skill.pluginDisplayName)"
  }

  static func disableDetail(_ skill: PluginSkillSummary) -> String {
    "Stops \(skill.pluginDisplayName) guidance from being added to future requests."
  }

  static func previewLine(_ preview: String?, maxLength: Int = 160) -> String? {
    guard let preview else {
      return nil
    }

    let line = preview
      .split(whereSeparator: \.isNewline)
      .map(String.init)
      .first?
      .trimmingCharacters(in: .whitespacesAndNewlines)

    guard let line, !line.isEmpty else {
      return nil
    }

    if line.count <= maxLength {
      return line
    }

    return "\(String(line.prefix(maxLength)))..."
  }

  static func issueText(_ skill: PluginSkillSummary) -> String? {
    guard skill.status != "ready" else {
      return nil
    }

    let blocker = skill.runBlocker?.trimmingCharacters(in: .whitespacesAndNewlines)
    let hint = skill.runRepairHint?.trimmingCharacters(in: .whitespacesAndNewlines)

    switch (nonEmpty(blocker), nonEmpty(hint)) {
    case let (blocker?, hint?):
      return "\(blocker) Fix: \(hint)"
    case let (blocker?, nil):
      return blocker
    case let (nil, hint?):
      return "Fix: \(hint)"
    case (nil, nil):
      return "This guidance needs review before Amentia can use it."
    }
  }

  private static func nonEmpty(_ value: String?) -> String? {
    guard let value, !value.isEmpty else {
      return nil
    }
    return value
  }
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
