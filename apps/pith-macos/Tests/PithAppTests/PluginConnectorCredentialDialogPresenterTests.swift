@testable import PithApp
import XCTest

final class PluginConnectorCredentialDialogPresenterTests: XCTestCase {
  func testNotionCredentialPromptExplainsLocalTokenSetup() {
    let connector = notionConnector()

    XCTAssertTrue(PluginConnectorCredentialDialogPresenter.requiresLocalSecret(connector))

    let prompt = PluginConnectorCredentialDialogPresenter.credentialPrompt(connector)
    XCTAssertTrue(prompt.contains("API key access for notion"))
    XCTAssertTrue(prompt.contains("Scopes: read_content, insert_content."))
    XCTAssertTrue(prompt.contains("A local token or API key is required"))
    XCTAssertTrue(prompt.contains("create an internal Notion integration"))
    XCTAssertTrue(prompt.contains("share every target parent page"))
    XCTAssertTrue(prompt.contains("passes it only to the local Notion connector runner"))
    XCTAssertTrue(prompt.contains("first publish still verifies"))
    XCTAssertTrue(prompt.contains("does not claim OAuth yet"))
    XCTAssertFalse(prompt.contains("Credential store"))

    let warning = PluginConnectorCredentialDialogPresenter.missingSecretWarningText(connector)
    XCTAssertTrue(warning.contains("Notion internal integration token"))
    XCTAssertTrue(warning.contains("share the target parent page"))
    XCTAssertTrue(warning.contains("during approved runs"))
  }

  func testSecretlessAuthorizationPromptRemainsAvailableForNonApiKeyConnectors() {
    let connector = PluginConnectorSummary(
      id: "local-marker::calendar",
      displayName: "Local Calendar",
      service: "calendar",
      pluginID: "local-marker",
      pluginDisplayName: "Local Marker",
      enabled: true,
      status: "ready",
      permissions: [],
      manifestPath: "/plugins/local-marker/pith-plugin.json",
      homepage: nil,
      authType: "none",
      authRequired: false,
      authScopes: [],
      credentialStore: "none",
      workflows: [],
      authStatus: "ready",
      credentialPresent: false,
      credentialSecretPresent: false,
      credentialProvider: nil,
      credentialHandle: nil,
      credentialLabel: nil,
      authorizedAt: nil,
      credentialUpdatedAt: nil
    )

    XCTAssertFalse(PluginConnectorCredentialDialogPresenter.requiresLocalSecret(connector))
    XCTAssertTrue(
      PluginConnectorCredentialDialogPresenter
        .credentialPrompt(connector)
        .contains("authorized without a token")
    )
    XCTAssertTrue(
      PluginConnectorCredentialDialogPresenter
        .missingSecretWarningText(connector)
        .contains("local token or API key")
    )
  }

  private func notionConnector() -> PluginConnectorSummary {
    PluginConnectorSummary(
      id: "notion-connector::notion",
      displayName: "Notion",
      service: "notion",
      pluginID: "notion-connector",
      pluginDisplayName: "Notion Connector",
      enabled: true,
      status: "needsAuth",
      permissions: ["network.outbound", "mcp.connect"],
      manifestPath: "/plugins/notion-connector/pith-plugin.json",
      homepage: "https://www.notion.so",
      authType: "api_key",
      authRequired: true,
      authScopes: ["read_content", "insert_content"],
      credentialStore: "local",
      workflows: [],
      authStatus: "needsAuth",
      credentialPresent: false,
      credentialSecretPresent: false,
      credentialProvider: nil,
      credentialHandle: nil,
      credentialLabel: nil,
      authorizedAt: nil,
      credentialUpdatedAt: nil
    )
  }
}
