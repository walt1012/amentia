import Foundation

struct PluginActionSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let hasRuntimeThreadSelection: Bool
  let selectedThreadID: String?
  let hasActiveOrPendingTurn: Bool
  let hasLifecycleOperation: Bool
  let plugins: [PluginSummary]
  let connectors: [PluginConnectorSummary]
  let commands: [PluginCommandSummary]
}

enum PluginActionPlanner {
  static func isRemovable(_ plugin: PluginSummary) -> Bool {
    plugin.provenance == "local"
  }

  static func canSetEnabled(pluginID: String, snapshot: PluginActionSnapshot) -> Bool {
    guard let plugin = snapshot.plugins.first(where: { $0.id == pluginID }) else {
      return false
    }

    return snapshot.runtimeState == .ready
      && plugin.status == "ready"
      && !snapshot.hasActiveOrPendingTurn
      && !snapshot.hasLifecycleOperation
  }

  static func canRemove(pluginID: String, snapshot: PluginActionSnapshot) -> Bool {
    guard let plugin = snapshot.plugins.first(where: { $0.id == pluginID }) else {
      return false
    }

    return snapshot.runtimeState == .ready
      && isRemovable(plugin)
      && !snapshot.hasActiveOrPendingTurn
      && !snapshot.hasLifecycleOperation
  }

  static func canAuthorizeConnector(
    connectorID: String,
    snapshot: PluginActionSnapshot
  ) -> Bool {
    connectorAuthorizeDisabledReason(connectorID: connectorID, snapshot: snapshot) == nil
  }

  static func connectorAuthorizeDisabledReason(
    connectorID: String,
    snapshot: PluginActionSnapshot
  ) -> String? {
    guard let connector = snapshot.connectors.first(where: { $0.id == connectorID }) else {
      return "Connection is not loaded."
    }

    if snapshot.runtimeState != .ready {
      return "Amentia is not ready."
    }
    if !connector.enabled {
      return "Connection is disabled."
    }
    if !connector.authRequired {
      return "Connection does not require authorization."
    }
    if !connectorNeedsAuthorization(connector) {
      return "Connection is already authorized."
    }
    if snapshot.hasActiveOrPendingTurn {
      return "Finish or cancel the active task first."
    }
    if snapshot.hasLifecycleOperation {
      return "Finish the current connection operation first."
    }

    return nil
  }

  static func canClearConnectorCredential(
    connectorID: String,
    snapshot: PluginActionSnapshot
  ) -> Bool {
    connectorClearCredentialDisabledReason(connectorID: connectorID, snapshot: snapshot) == nil
  }

  static func connectorClearCredentialDisabledReason(
    connectorID: String,
    snapshot: PluginActionSnapshot
  ) -> String? {
    guard let connector = snapshot.connectors.first(where: { $0.id == connectorID }) else {
      return "Connection is not loaded."
    }

    if snapshot.runtimeState != .ready {
      return "Amentia is not ready."
    }
    if !connector.credentialPresent {
      return "Connection has no saved token or key."
    }
    if snapshot.hasActiveOrPendingTurn {
      return "Finish or cancel the active task first."
    }
    if snapshot.hasLifecycleOperation {
      return "Finish the current connection operation first."
    }

    return nil
  }

  static func commandNeedsExecutionContract(
    commandID: String,
    snapshot: PluginActionSnapshot
  ) -> Bool {
    guard let command = snapshot.commands.first(where: { $0.id == commandID }) else {
      return true
    }

    return command.runStatus == "missingExecution"
      || command.runStatus == "unsupportedExecution"
      || command.execution == nil
      || command.execution?.supported == false
  }

  static func commandNeedsConnectorAuth(
    commandID: String,
    snapshot: PluginActionSnapshot
  ) -> Bool {
    guard let command = snapshot.commands.first(where: { $0.id == commandID }) else {
      return false
    }

    return command.runStatus == "needsConnectorAuth"
  }

  static func commandAuthorizationConnectorID(
    commandID: String,
    snapshot: PluginActionSnapshot
  ) -> String? {
    guard let command = snapshot.commands.first(where: { $0.id == commandID }),
          command.runStatus == "needsConnectorAuth"
    else {
      return nil
    }

    let candidateConnectorIds = command.requiredConnectorIds.isEmpty
      ? command.declaredConnectorIds
      : command.requiredConnectorIds
    return candidateConnectorIds.first { connectorID in
      guard let connector = snapshot.connectors.first(where: { $0.id == connectorID }) else {
        return false
      }

      return connector.enabled
        && connector.authRequired
        && connectorNeedsAuthorization(connector)
    }
  }

  static func commandRunBlocker(commandID: String, snapshot: PluginActionSnapshot) -> String? {
    snapshot.commands.first(where: { $0.id == commandID })?.runBlocker
  }

  private static func connectorNeedsAuthorization(
    _ connector: PluginConnectorSummary
  ) -> Bool {
    PluginStatusDisplay.authorizationStatus(
      connector.authStatus,
      authRequired: connector.authRequired,
      credentialPresent: connector.credentialPresent,
      credentialSecretPresent: connector.credentialSecretPresent
    ) == "needs sign in"
  }

  static func canRunCommand(commandID: String, snapshot: PluginActionSnapshot) -> Bool {
    commandRunDisabledReason(commandID: commandID, snapshot: snapshot) == nil
  }

  static func canRunCommandWithInput(commandID: String, snapshot: PluginActionSnapshot) -> Bool {
    guard let command = snapshot.commands.first(where: { $0.id == commandID }),
          command.acceptsPlainInput
    else {
      return false
    }

    return commandRunDisabledReason(commandID: commandID, snapshot: snapshot) == nil
  }

  static func commandRunDisabledReason(
    commandID: String,
    snapshot: PluginActionSnapshot
  ) -> String? {
    guard let command = snapshot.commands.first(where: { $0.id == commandID }) else {
      return "Action was not found."
    }

    if command.runStatus != "ready" {
      return command.runBlocker ?? "Action is not ready."
    }
    guard command.execution?.supported == true else {
      return "Action needs a supported local runner."
    }

    if !command.unsupportedRequiredInputFieldNames.isEmpty {
      return "Action requires unsupported input fields: \(command.unsupportedRequiredInputFieldNames.joined(separator: ", "))."
    }
    if command.requiresConnectorInput && command.declaredConnectorIds.isEmpty {
      return "Action requires connection input, but no connection is declared."
    }
    if snapshot.runtimeState != .ready {
      return "Amentia is not ready."
    }
    if command.requiresWorkspaceInput && !snapshot.hasRuntimeThreadSelection {
      return "Action requires a project-bound session."
    }
    if !snapshot.hasRuntimeThreadSelection || snapshot.selectedThreadID == nil {
      return "Select or create a session first."
    }
    if snapshot.hasActiveOrPendingTurn {
      return "Finish or cancel the active task first."
    }
    if snapshot.hasLifecycleOperation {
      return "Finish the current plugin operation first."
    }

    return nil
  }

  static func directCommandRunDisabledReason(
    commandID: String,
    snapshot: PluginActionSnapshot
  ) -> String? {
    if let reason = commandRunDisabledReason(commandID: commandID, snapshot: snapshot) {
      return reason
    }

    guard let command = snapshot.commands.first(where: { $0.id == commandID }) else {
      return "Action is not loaded."
    }
    if command.requiresPlainInput {
      return "Action requires input. Use Run with Input."
    }

    return nil
  }
}
