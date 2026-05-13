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
    guard let connector = snapshot.connectors.first(where: { $0.id == connectorID }) else {
      return false
    }

    return snapshot.runtimeState == .ready
      && connector.enabled
      && connector.authRequired
      && connector.authStatus == "needsAuth"
      && !snapshot.hasActiveOrPendingTurn
      && !snapshot.hasLifecycleOperation
  }

  static func canClearConnectorCredential(
    connectorID: String,
    snapshot: PluginActionSnapshot
  ) -> Bool {
    guard let connector = snapshot.connectors.first(where: { $0.id == connectorID }) else {
      return false
    }

    return snapshot.runtimeState == .ready
      && connector.credentialPresent
      && !snapshot.hasActiveOrPendingTurn
      && !snapshot.hasLifecycleOperation
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
    guard let command = snapshot.commands.first(where: { $0.id == commandID }),
          command.execution?.supported == true
    else {
      return "Command needs a supported execution contract."
    }

    if snapshot.runtimeState != .ready {
      return "Runtime is not ready."
    }
    if !snapshot.isLocalModelReady {
      return "Local model is not ready."
    }
    if command.runStatus != "ready" {
      return command.runBlocker ?? "Command is not ready."
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
}
