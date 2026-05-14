import Foundation

struct PluginActionSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let isLocalModelReady: Bool
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
      return "Connector is not loaded."
    }

    if snapshot.runtimeState != .ready {
      return "Runtime is not ready."
    }
    if !connector.enabled {
      return "Connector plugin is disabled."
    }
    if !connector.authRequired {
      return "Connector does not require authorization."
    }
    if connector.authStatus != "needsAuth" {
      return "Connector is already authorized."
    }
    if snapshot.hasActiveOrPendingTurn {
      return "Finish or cancel the active task first."
    }
    if snapshot.hasLifecycleOperation {
      return "Finish the current plugin operation first."
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
      return "Connector is not loaded."
    }

    if snapshot.runtimeState != .ready {
      return "Runtime is not ready."
    }
    if !connector.credentialPresent {
      return "Connector has no stored credential."
    }
    if snapshot.hasActiveOrPendingTurn {
      return "Finish or cancel the active task first."
    }
    if snapshot.hasLifecycleOperation {
      return "Finish the current plugin operation first."
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
        && connector.authStatus == "needsAuth"
    }
  }

  static func commandRunBlocker(commandID: String, snapshot: PluginActionSnapshot) -> String? {
    snapshot.commands.first(where: { $0.id == commandID })?.runBlocker
  }

  static func canRunCommand(commandID: String, snapshot: PluginActionSnapshot) -> Bool {
    commandRunDisabledReason(commandID: commandID, snapshot: snapshot) == nil
  }

  static func commandRunDisabledReason(
    commandID: String,
    snapshot: PluginActionSnapshot
  ) -> String? {
    guard let command = snapshot.commands.first(where: { $0.id == commandID }) else {
      return "Command was not found."
    }

    if command.runStatus != "ready" {
      return command.runBlocker ?? "Command is not ready."
    }
    guard command.execution?.supported == true else {
      return "Command needs a supported execution contract."
    }

    if !command.unsupportedRequiredInputFieldNames.isEmpty {
      return "Command requires unsupported input fields: \(command.unsupportedRequiredInputFieldNames.joined(separator: ", "))."
    }
    if command.requiresConnectorInput && command.declaredConnectorIds.isEmpty {
      return "Command requires connector input, but no connector is declared."
    }
    if snapshot.runtimeState != .ready {
      return "Runtime is not ready."
    }
    if !snapshot.isLocalModelReady {
      return "Local model is not ready."
    }
    if command.requiresWorkspaceInput && !snapshot.hasRuntimeThreadSelection {
      return "Command requires a workspace-bound thread."
    }
    if !snapshot.hasRuntimeThreadSelection || snapshot.selectedThreadID == nil {
      return "Select or create a thread first."
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
      return "Command is not loaded."
    }
    if command.requiresPlainInput {
      return "Command requires input. Use Run with Input."
    }

    return nil
  }
}
