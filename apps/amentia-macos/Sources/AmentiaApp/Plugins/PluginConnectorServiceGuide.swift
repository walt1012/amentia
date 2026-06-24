import Foundation

enum PluginConnectorServiceGuide {
  static func setupPrompt(connector: PluginConnectorSummary) -> String? {
    switch normalizedService(connector.service) {
    case "notion":
      return [
        "",
        "",
        "Notion setup: create an internal Notion integration and copy its internal integration token.",
        "Paste that token, then share every target parent page with the integration before publishing.",
        "Amentia keeps the token local, passes it only to the local Notion plugin runner, and does not claim OAuth yet.",
        "Authorization stores the token; the first publish still verifies the token, page sharing, and Notion response confirmation.",
      ].joined(separator: "\n")
    default:
      return nil
    }
  }

  static func missingTokenOrKeyWarning(connector: PluginConnectorSummary) -> String? {
    switch normalizedService(connector.service) {
    case "notion":
      return [
        "Paste the Notion internal integration token before authorizing this connection.",
        "If you have not created one yet, create an internal Notion integration first and share the target parent page with it.",
        "Amentia keeps the token local and passes it only to the Notion plugin runner during approved runs.",
      ].joined(separator: " ")
    default:
      return nil
    }
  }

  static func defaultCredentialLabel(connector: PluginConnectorSummary) -> String? {
    switch normalizedService(connector.service) {
    case "notion":
      return "Local Notion integration token"
    default:
      return nil
    }
  }

  static func tokenOrKeyPlaceholder(connector: PluginConnectorSummary) -> String? {
    switch normalizedService(connector.service) {
    case "notion":
      return "Paste the Notion internal integration token"
    default:
      return nil
    }
  }

  static func commandInputAppendix(command: PluginCommandSummary) -> String? {
    guard commandTargetsService("notion", command: command),
          commandLooksLikePublish(command)
    else {
      return nil
    }

    return [
      "Input: a Notion parent page URL, parent page ID, or saved parent alias, with optional title and body.",
      "If you paste JSON, use parentPageId for the parent page.",
      "You can paste only the parent page URL to use the default title.",
      "The parent page must be shared with the Notion integration before publishing.",
      "Amentia still requests approval before the external action.",
    ].joined(separator: " ")
  }

  static func receiptKindLabel(_ value: String) -> String? {
    switch value {
    case "notionApiResponse":
      return "Notion confirmation"
    default:
      return nil
    }
  }

  private static func commandTargetsService(
    _ service: String,
    command: PluginCommandSummary
  ) -> Bool {
    let normalizedTarget = normalizedService(service)
    if let workflowService = command.execution?.workflow?.service,
       normalizedService(workflowService) == normalizedTarget
    {
      return true
    }

    let searchable = [
      command.id,
      command.execution?.kind,
      command.executionKind,
    ]
    .compactMap { $0 }
    .joined(separator: " ")
    .lowercased()

    return searchable.contains(normalizedTarget)
  }

  private static func commandLooksLikePublish(_ command: PluginCommandSummary) -> Bool {
    let searchable = [
      command.id,
      command.title,
      command.execution?.kind,
      command.executionKind,
      command.execution?.workflow?.action,
    ]
    .compactMap { $0 }
    .joined(separator: " ")
    .lowercased()

    return searchable.contains("publish") || searchable.contains("createpage")
  }

  private static func normalizedService(_ service: String) -> String {
    service
      .trimmingCharacters(in: .whitespacesAndNewlines)
      .lowercased()
      .replacingOccurrences(of: "_", with: "-")
  }
}
