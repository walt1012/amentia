import Foundation

enum PluginStateLoader {
  static func refresh(using runtimeBridge: RuntimeBridge) async -> PluginStateRefresh {
    guard !Task.isCancelled else {
      return emptyRefresh()
    }

    let catalogRefresh = await load("catalog refresh") {
      try await runtimeBridge.refreshPluginCatalog()
    }
    guard !Task.isCancelled else {
      return emptyRefresh()
    }
    let pluginLoad: (
      value: [RuntimeBridge.RuntimePlugin]?,
      diagnostic: String?,
      recoveryAttributes: [String: String]
    )
    if let refresh = catalogRefresh.value {
      pluginLoad = (
        refresh.plugins,
        refresh.stateWarning.map { "catalog state: \($0)" },
        [:]
      )
    } else {
      pluginLoad = await load("catalog") {
        try await runtimeBridge.listPlugins()
      }
    }
    guard !Task.isCancelled else {
      return emptyRefresh()
    }
    let registryLoad = await load("capability registry") {
      try await runtimeBridge.pluginCapabilityRegistry()
    }
    guard !Task.isCancelled else {
      return emptyRefresh()
    }
    let commandLoad = await load("command registry") {
      try await runtimeBridge.listPluginCommands()
    }
    guard !Task.isCancelled else {
      return emptyRefresh()
    }
    let connectorLoad = await load("connector registry") {
      try await runtimeBridge.listPluginConnectors()
    }
    guard !Task.isCancelled else {
      return emptyRefresh()
    }
    let hookLoad = await load("hook registry") {
      try await runtimeBridge.listPluginHooks()
    }
    let runtimePlugins = pluginLoad.value
    let runtimeRegistry = registryLoad.value
    let runtimeCommands = commandLoad.value
    let runtimeConnectors = connectorLoad.value
    let runtimeHooks = hookLoad.value
    let diagnostics = [
      catalogRefresh.diagnostic,
      pluginLoad.diagnostic,
      registryLoad.diagnostic,
      commandLoad.diagnostic,
      connectorLoad.diagnostic,
      hookLoad.diagnostic,
    ].compactMap { $0 }

    let plugins = runtimePlugins.map { plugins in
      plugins.map { RuntimeSummaryMapper.pluginSummary(from: $0) }
    }
    let registryState = buildRegistryState(runtimeRegistry: runtimeRegistry, plugins: plugins)

    return PluginStateRefresh(
      plugins: plugins,
      registrySummary: registryState.summary,
      capabilities: registryState.capabilities,
      connectors: mappedConnectors(runtimeConnectors, pluginsLoaded: runtimePlugins != nil),
      commands: mappedCommands(runtimeCommands, pluginsLoaded: runtimePlugins != nil),
      hooks: mappedHooks(runtimeHooks, pluginsLoaded: runtimePlugins != nil),
      diagnostics: diagnostics,
      refreshRecoveryAttributes: catalogRefresh.recoveryAttributes
    )
  }

  private static func load<T>(
    _ label: String,
    operation: () async throws -> T
  ) async -> (value: T?, diagnostic: String?, recoveryAttributes: [String: String]) {
    guard !Task.isCancelled else {
      return (nil, nil, [:])
    }

    do {
      let value = try await operation()
      guard !Task.isCancelled else {
        return (nil, nil, [:])
      }
      return (value, nil, [:])
    } catch is CancellationError {
      return (nil, nil, [:])
    } catch {
      guard !Task.isCancelled else {
        return (nil, nil, [:])
      }
      return (
        nil,
        "\(label): \(error.localizedDescription)",
        runtimeRecoveryAttributes(from: error)
      )
    }
  }

  private static func emptyRefresh() -> PluginStateRefresh {
    PluginStateRefresh(
      plugins: nil,
      registrySummary: nil,
      capabilities: nil,
      connectors: nil,
      commands: nil,
      hooks: nil,
      diagnostics: [],
      refreshRecoveryAttributes: [:]
    )
  }

  private static func runtimeRecoveryAttributes(from error: Error) -> [String: String] {
    guard let runtimeError = error as? RuntimeBridge.RuntimeError else {
      return [:]
    }

    return runtimeError.recoveryAttributes
  }

  private static func buildRegistryState(
    runtimeRegistry: RuntimeBridge.RuntimePluginCapabilityRegistry?,
    plugins: [PluginSummary]?
  ) -> (
    summary: PluginCapabilityRegistrySummary?,
    capabilities: [PluginCapabilitySummary]?
  ) {
    if let runtimeRegistry {
      return (
        RuntimeSummaryMapper.pluginRegistrySummary(from: runtimeRegistry.summary),
        runtimeRegistry.capabilities.map { RuntimeSummaryMapper.pluginCapabilitySummary(from: $0) }
      )
    }

    guard let plugins else {
      return (nil, nil)
    }

    return (
      PluginCapabilityRegistrySummary(
        enabledPluginCount: plugins.filter { $0.status == "ready" && $0.enabled }.count,
        totalCapabilityCount: 0,
        capabilityCountsByKind: [:]
      ),
      []
    )
  }

  private static func mappedConnectors(
    _ connectors: [RuntimeBridge.RuntimePluginConnector]?,
    pluginsLoaded: Bool
  ) -> [PluginConnectorSummary]? {
    if let connectors {
      return connectors.map { RuntimeSummaryMapper.pluginConnectorSummary(from: $0) }
    }

    return pluginsLoaded ? [] : nil
  }

  private static func mappedCommands(
    _ commands: [RuntimeBridge.RuntimePluginCommand]?,
    pluginsLoaded: Bool
  ) -> [PluginCommandSummary]? {
    if let commands {
      return commands.map { RuntimeSummaryMapper.pluginCommandSummary(from: $0) }
    }

    return pluginsLoaded ? [] : nil
  }

  private static func mappedHooks(
    _ hooks: [RuntimeBridge.RuntimePluginHook]?,
    pluginsLoaded: Bool
  ) -> [PluginHookSummary]? {
    if let hooks {
      return hooks.map { RuntimeSummaryMapper.pluginHookSummary(from: $0) }
    }

    return pluginsLoaded ? [] : nil
  }
}
