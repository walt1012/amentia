import Foundation

struct PluginStateRefresh {
  let plugins: [PluginSummary]?
  let registrySummary: PluginCapabilityRegistrySummary?
  let capabilities: [PluginCapabilitySummary]?
  let connectors: [PluginConnectorSummary]?
  let commands: [PluginCommandSummary]?
  let hooks: [PluginHookSummary]?
  let diagnostics: [String]
}

struct PluginRuntimeState {
  var plugins: [PluginSummary]
  var registrySummary: PluginCapabilityRegistrySummary?
  var capabilities: [PluginCapabilitySummary]
  var connectors: [PluginConnectorSummary]
  var commands: [PluginCommandSummary]
  var hooks: [PluginHookSummary]
  var diagnostics: [String]
  private var lifecycleOperationID: UUID?

  init(
    plugins: [PluginSummary] = [],
    registrySummary: PluginCapabilityRegistrySummary? = nil,
    capabilities: [PluginCapabilitySummary] = [],
    connectors: [PluginConnectorSummary] = [],
    commands: [PluginCommandSummary] = [],
    hooks: [PluginHookSummary] = [],
    diagnostics: [String] = [],
    lifecycleOperationID: UUID? = nil
  ) {
    self.plugins = plugins
    self.registrySummary = registrySummary
    self.capabilities = capabilities
    self.connectors = connectors
    self.commands = commands
    self.hooks = hooks
    self.diagnostics = diagnostics
    self.lifecycleOperationID = lifecycleOperationID
  }

  var hasLifecycleOperation: Bool {
    lifecycleOperationID != nil
  }

  var dashboardSnapshot: PluginDashboardSnapshot {
    PluginDashboardSnapshot(
      plugins: plugins,
      registrySummary: registrySummary,
      capabilities: capabilities,
      connectors: connectors,
      commands: commands,
      hooks: hooks,
      diagnostics: diagnostics,
      hasLifecycleOperation: hasLifecycleOperation
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
    diagnostics = refresh.diagnostics
  }

  mutating func beginLifecycleOperation() -> UUID? {
    guard lifecycleOperationID == nil else {
      return nil
    }

    let operationID = UUID()
    lifecycleOperationID = operationID
    return operationID
  }

  mutating func finishLifecycleOperation(_ operationID: UUID) {
    guard lifecycleOperationID == operationID else {
      return
    }

    lifecycleOperationID = nil
  }

  mutating func reset() {
    self = PluginRuntimeState()
  }
}
