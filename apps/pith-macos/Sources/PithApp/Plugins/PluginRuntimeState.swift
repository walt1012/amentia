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

  var dashboardSnapshot: PluginDashboardSnapshot {
    PluginDashboardSnapshot(
      plugins: plugins,
      registrySummary: registrySummary,
      capabilities: capabilities,
      connectors: connectors,
      commands: commands,
      hooks: hooks
    )
  }

  func plugin(id: String) -> PluginSummary? {
    plugins.first { $0.id == id }
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
