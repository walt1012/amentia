@testable import AmentiaApp
import XCTest

final class PluginConnectorCredentialDialogPresenterTests: XCTestCase {
  func testNotionCredentialPromptExplainsLocalTokenSetup() {
    let connector = notionConnector()

    XCTAssertTrue(PluginConnectorCredentialDialogPresenter.requiresLocalTokenOrKey(connector))

    let prompt = PluginConnectorCredentialDialogPresenter.credentialPrompt(connector)
    XCTAssertTrue(prompt.contains("API key access for Notion"))
    XCTAssertTrue(prompt.contains("Access: read content, create content."))
    XCTAssertTrue(prompt.contains("Paste a local token or API key"))
    XCTAssertTrue(prompt.contains("create an internal Notion integration"))
    XCTAssertTrue(prompt.contains("share every target parent page"))
    XCTAssertTrue(prompt.contains("passes it only to the local Notion plugin runner"))
    XCTAssertTrue(prompt.contains("first publish still verifies"))
    XCTAssertTrue(prompt.contains("does not claim OAuth yet"))
    XCTAssertFalse(prompt.contains("Credential store"))
    XCTAssertFalse(prompt.contains("Credential label"))
    XCTAssertFalse(prompt.contains("Secret"))

    let warning = PluginConnectorCredentialDialogPresenter.missingTokenOrKeyWarningText(connector)
    XCTAssertTrue(warning.contains("Notion internal integration token"))
    XCTAssertTrue(warning.contains("share the target parent page"))
    XCTAssertTrue(warning.contains("during approved runs"))
  }

  func testApiKeyAuthSpellingStillRequiresLocalTokenOrKey() {
    let connector = notionConnector(authType: "apiKey")

    XCTAssertTrue(PluginConnectorCredentialDialogPresenter.requiresLocalTokenOrKey(connector))
    XCTAssertTrue(
      PluginConnectorCredentialDialogPresenter
        .credentialPrompt(connector)
        .contains("API key access for Notion")
    )
  }

  func testCredentialDialogCopyUsesUserFacingLabels() {
    let connector = notionConnector()

    XCTAssertEqual(PluginConnectorCredentialDialogPresenter.labelFieldTitle, "Name")
    XCTAssertEqual(PluginConnectorCredentialDialogPresenter.tokenOrKeyFieldTitle, "Token or key")
    XCTAssertEqual(
      PluginConnectorCredentialDialogPresenter.defaultCredentialLabel(connector),
      "Local Notion integration token"
    )
    XCTAssertEqual(
      PluginConnectorCredentialDialogPresenter.tokenOrKeyPlaceholder(connector),
      "Paste the Notion internal integration token"
    )
  }

  func testServiceGuideAddsNotionCommandInputHelpFromWorkflowService() {
    let command = PluginCommandSummary(
      id: "pages.create",
      title: "Create Page",
      description: "Create a remote page.",
      pluginID: "notion-connector",
      pluginDisplayName: "Notion Connector",
      permissions: ["network.outbound"],
      sourcePath: "/plugins/notion-connector/amentia-plugin.json",
      execution: PluginCommandExecutionSummary(
        kind: "mcp.remote.createPage",
        driver: "node",
        entrypoint: nil,
        workflowID: "notion.create-page",
        workflow: PluginCommandWorkflowSummary(
          workflowID: "notion.create-page",
          displayName: "Create Page",
          connectorID: "notion-connector::notion",
          service: "notion",
          action: "createPage",
          maxAgentSteps: 2,
          stages: ["inspectBeforeWrite", "completed"],
          statuses: ["inspected", "success"],
          commandIDs: ["pages.create"]
        ),
        input: PluginCommandEnvelopeSummary(envelope: "text", fields: [
          PluginCommandEnvelopeFieldSummary(
            name: "input",
            kind: "text",
            required: true,
            description: "Page request."
          ),
        ]),
        output: PluginCommandEnvelopeSummary(envelope: "json", fields: []),
        supported: true
      ),
      executionKind: "mcp.remote.createPage",
      memorySummary: nil,
      runStatus: "ready",
      runBlocker: nil,
      runRepairHint: nil,
      declaredConnectorIds: ["notion-connector::notion"],
      requiredConnectorIds: ["notion-connector::notion"],
      approvalRequired: true,
      approvalReason: "Connection access"
    )

    let prompt = PluginCommandInputDialogPresenter.inputPrompt(command, override: nil)

    XCTAssertTrue(prompt.contains("Page request."))
    XCTAssertTrue(prompt.contains("parentPageId"))
    XCTAssertTrue(prompt.contains("Notion integration"))
    XCTAssertTrue(prompt.contains("approval before the external action"))
    XCTAssertFalse(prompt.contains("notion-connector::notion"))
  }

  func testServiceGuideLeavesGenericCommandInputUnchanged() {
    let command = PluginCommandSummary(
      id: "calendar.create",
      title: "Create Event",
      description: "Create a local event.",
      pluginID: "calendar",
      pluginDisplayName: "Calendar",
      permissions: [],
      sourcePath: "/plugins/calendar/amentia-plugin.json",
      execution: nil,
      executionKind: nil,
      memorySummary: nil,
      runStatus: "ready",
      runBlocker: nil,
      runRepairHint: nil,
      declaredConnectorIds: [],
      requiredConnectorIds: [],
      approvalRequired: false,
      approvalReason: nil
    )

    let prompt = PluginCommandInputDialogPresenter.inputPrompt(command, override: nil)

    XCTAssertEqual(prompt, "Pass a short text input to this action.")
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
      manifestPath: "/plugins/local-marker/amentia-plugin.json",
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

    XCTAssertFalse(PluginConnectorCredentialDialogPresenter.requiresLocalTokenOrKey(connector))
    XCTAssertTrue(
      PluginConnectorCredentialDialogPresenter
        .credentialPrompt(connector)
        .contains("can be approved without a token")
    )
    XCTAssertFalse(
      PluginConnectorCredentialDialogPresenter
        .credentialPrompt(connector)
        .contains("secret")
    )
    XCTAssertTrue(
      PluginConnectorCredentialDialogPresenter
        .missingTokenOrKeyWarningText(connector)
        .contains("local token or API key")
    )
  }

  private func notionConnector(authType: String = "api_key") -> PluginConnectorSummary {
    PluginConnectorSummary(
      id: "notion-connector::notion",
      displayName: "Notion",
      service: "notion",
      pluginID: "notion-connector",
      pluginDisplayName: "Notion Connector",
      enabled: true,
      status: "needsAuth",
      permissions: ["network.outbound", "mcp.connect"],
      manifestPath: "/plugins/notion-connector/amentia-plugin.json",
      homepage: "https://www.notion.so",
      authType: authType,
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
