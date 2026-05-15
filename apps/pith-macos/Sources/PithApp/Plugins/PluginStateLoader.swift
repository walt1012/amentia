import Foundation

enum PluginStateLoader {
  static func refresh(using runtimeBridge: RuntimeBridge) async -> PluginStateRefresh {
    let catalogRefresh = await load("catalog refresh") {
      try await runtimeBridge.refreshPluginCatalog()
    }
    let pluginLoad: (value: [RuntimeBridge.RuntimePlugin]?, diagnostic: String?)
    if let refresh = catalogRefresh.value {
      pluginLoad = (
        refresh.plugins,
        refresh.stateWarning.map { "catalog state: \($0)" }
      )
    } else {
      pluginLoad = await load("catalog") {
        try await runtimeBridge.listPlugins()
      }
    }
    let registryLoad = await load("capability registry") {
      try await runtimeBridge.pluginCapabilityRegistry()
    }
    let commandLoad = await load("command registry") {
      try await runtimeBridge.listPluginCommands()
    }
    let connectorLoad = await load("connector registry") {
      try await runtimeBridge.listPluginConnectors()
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
      diagnostics: diagnostics
    )
  }

  private static func load<T>(
    _ label: String,
    operation: () async throws -> T
  ) async -> (value: T?, diagnostic: String?) {
    do {
      return (try await operation(), nil)
    } catch {
      return (nil, "\(label): \(error.localizedDescription)")
    }
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
