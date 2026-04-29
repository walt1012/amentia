import Foundation

struct PluginStateRefresh {
  let plugins: [PluginSummary]?
  let registrySummary: PluginCapabilityRegistrySummary?
  let capabilities: [PluginCapabilitySummary]?
  let connectors: [PluginConnectorSummary]?
  let commands: [PluginCommandSummary]?
  let hooks: [PluginHookSummary]?
}

struct PluginRuntimeState {
  var plugins: [PluginSummary]
  var registrySummary: PluginCapabilityRegistrySummary?
  var capabilities: [PluginCapabilitySummary]
  var connectors: [PluginConnectorSummary]
  var commands: [PluginCommandSummary]
  var hooks: [PluginHookSummary]

  init(
    plugins: [PluginSummary] = [],
    registrySummary: PluginCapabilityRegistrySummary? = nil,
    capabilities: [PluginCapabilitySummary] = [],
    connectors: [PluginConnectorSummary] = [],
    commands: [PluginCommandSummary] = [],
    hooks: [PluginHookSummary] = []
  ) {
    self.plugins = plugins
    self.registrySummary = registrySummary
    self.capabilities = capabilities
    self.connectors = connectors
    self.commands = commands
    self.hooks = hooks
  }

  mutating func apply(_ refresh: PluginStateRefresh) {
    if let plugins = refresh.plugins {
      self.plugins = plugins
    }
    if let registrySummary = refresh.registrySummary {
      self.registrySummary = registrySummary
    }
    if let capabilities = refresh.capabilities {
      self.capabilities = capabilities
    }
    if let connectors = refresh.connectors {
      self.connectors = connectors
    }
    if let commands = refresh.commands {
      self.commands = commands
    }
    if let hooks = refresh.hooks {
      self.hooks = hooks
    }
  }

  mutating func reset() {
    self = PluginRuntimeState()
  }
}

enum PluginStateLoader {
  static func refresh(using runtimeBridge: RuntimeBridge) async -> PluginStateRefresh {
    let runtimePlugins = try? await runtimeBridge.listPlugins()
    let runtimeRegistry = try? await runtimeBridge.pluginCapabilityRegistry()
    let runtimeCommands = try? await runtimeBridge.listPluginCommands()
    let runtimeConnectors = try? await runtimeBridge.listPluginConnectors()
    let runtimeHooks = try? await runtimeBridge.listPluginHooks()

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
      hooks: mappedHooks(runtimeHooks, pluginsLoaded: runtimePlugins != nil)
    )
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
