import Foundation

struct PluginActionSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let isLocalModelReady: Bool
  let hasRuntimeThreadSelection: Bool
  let selectedThreadID: String?
  let hasActiveOrPendingTurn: Bool
  let plugins: [PluginSummary]
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
  }

  static func canRemove(pluginID: String, snapshot: PluginActionSnapshot) -> Bool {
    guard let plugin = snapshot.plugins.first(where: { $0.id == pluginID }) else {
      return false
    }

    return snapshot.runtimeState == .ready
      && isRemovable(plugin)
      && !snapshot.hasActiveOrPendingTurn
  }

  static func commandNeedsExecutionContract(
    commandID: String,
    snapshot: PluginActionSnapshot
  ) -> Bool {
    guard let command = snapshot.commands.first(where: { $0.id == commandID }) else {
      return true
    }

    return command.executionKind == nil
  }

  static func canRunCommand(commandID: String, snapshot: PluginActionSnapshot) -> Bool {
    guard let command = snapshot.commands.first(where: { $0.id == commandID }),
          command.executionKind != nil
    else {
      return false
    }

    return snapshot.runtimeState == .ready
      && snapshot.isLocalModelReady
      && snapshot.hasRuntimeThreadSelection
      && snapshot.selectedThreadID != nil
      && !snapshot.hasActiveOrPendingTurn
  }
}
