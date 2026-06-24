import Foundation

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
      return "Workflow ready: \(workflow.workflowLabel)"
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
