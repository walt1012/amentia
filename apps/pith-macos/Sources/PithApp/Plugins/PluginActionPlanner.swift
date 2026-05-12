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

    return command.execution == nil || command.execution?.supported == false
  }

  static func canRunCommand(commandID: String, snapshot: PluginActionSnapshot) -> Bool {
    guard let command = snapshot.commands.first(where: { $0.id == commandID }),
          command.execution?.supported == true
    else {
      return false
    }

    return snapshot.runtimeState == .ready
      && snapshot.isLocalModelReady
      && snapshot.hasRuntimeThreadSelection
      && snapshot.selectedThreadID != nil
      && !snapshot.hasActiveOrPendingTurn
      && !snapshot.hasLifecycleOperation
  }
}
