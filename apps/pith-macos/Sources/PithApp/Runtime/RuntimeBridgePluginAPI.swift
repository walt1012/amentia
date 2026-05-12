import Foundation

extension RuntimeBridge {
  func listPlugins() async throws -> [RuntimePlugin] {
    let response: JSONRPCResponse<PluginListResult> = try await sendRequest(
      method: "plugin/list",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return result.plugins.map { plugin in
      RuntimePlugin(
        id: plugin.id,
        name: plugin.name,
        version: plugin.version,
        displayName: plugin.displayName,
        status: plugin.status,
        description: plugin.description,
        authorName: plugin.authorName,
        enabled: plugin.enabled,
        defaultEnabled: plugin.defaultEnabled,
        capabilities: plugin.capabilities,
        permissions: plugin.permissions,
        manifestPath: plugin.manifestPath,
        provenance: plugin.provenance,
        validationError: plugin.validationError,
        validationHint: plugin.validationHint
      )
    }
  }

  func installPlugin(sourcePath: String) async throws -> RuntimePlugin {
    let response: JSONRPCResponse<PluginInstallResult> = try await sendRequest(
      method: "plugin/install",
      params: PluginInstallParams(sourcePath: sourcePath)
    )
    let result = try responseResult(from: response)

    return RuntimePlugin(
      id: result.plugin.id,
      name: result.plugin.name,
      version: result.plugin.version,
      displayName: result.plugin.displayName,
      status: result.plugin.status,
      description: result.plugin.description,
      authorName: result.plugin.authorName,
      enabled: result.plugin.enabled,
      defaultEnabled: result.plugin.defaultEnabled,
      capabilities: result.plugin.capabilities,
      permissions: result.plugin.permissions,
      manifestPath: result.plugin.manifestPath,
      provenance: result.plugin.provenance,
      validationError: result.plugin.validationError,
      validationHint: result.plugin.validationHint
    )
  }

  func pluginCapabilityRegistry() async throws -> RuntimePluginCapabilityRegistry {
    let response: JSONRPCResponse<PluginCapabilityRegistryResult> = try await sendRequest(
      method: "plugin/capabilityRegistry",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return RuntimePluginCapabilityRegistry(
      capabilities: result.capabilities.map { capability in
        RuntimePluginCapability(
          capabilityID: capability.capabilityId,
          kind: capability.kind,
          identifier: capability.identifier,
          pluginID: capability.pluginId,
          pluginDisplayName: capability.pluginDisplayName,
          permissions: capability.permissions,
          manifestPath: capability.manifestPath,
          metadata: capability.metadata ?? [:]
        )
      },
      summary: RuntimePluginCapabilityRegistrySummary(
        enabledPluginCount: result.summary.enabledPluginCount,
        totalCapabilityCount: result.summary.totalCapabilityCount,
        capabilityCountsByKind: result.summary.capabilityCountsByKind
      )
    )
  }

  func listPluginCommands() async throws -> [RuntimePluginCommand] {
    let response: JSONRPCResponse<PluginCommandRegistryResult> = try await sendRequest(
      method: "plugin/commandRegistry",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return result.commands.map { command in
      RuntimePluginCommand(
        commandID: command.commandId,
        title: command.title,
        description: command.description,
        pluginID: command.pluginId,
        pluginDisplayName: command.pluginDisplayName,
        permissions: command.permissions,
        sourcePath: command.sourcePath,
        execution: command.execution.map {
          RuntimePluginCommandExecution(
            kind: $0.kind,
            driver: $0.driver,
            entrypoint: $0.entrypoint,
            supported: $0.supported
          )
        },
        executionKind: command.executionKind,
        memorySummary: command.memorySummary
      )
    }
  }

  func listPluginConnectors() async throws -> [RuntimePluginConnector] {
    let response: JSONRPCResponse<PluginConnectorRegistryResult> = try await sendRequest(
      method: "plugin/connectorRegistry",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return result.connectors.map { connector in
      RuntimePluginConnector(
        connectorID: connector.connectorId,
        displayName: connector.displayName,
        service: connector.service,
        pluginID: connector.pluginId,
        pluginDisplayName: connector.pluginDisplayName,
        enabled: connector.enabled,
        status: connector.status,
        permissions: connector.permissions,
        manifestPath: connector.manifestPath,
        homepage: connector.homepage,
        authType: connector.authType,
        authRequired: connector.authRequired,
        authScopes: connector.authScopes,
        credentialStore: connector.credentialStore
      )
    }
  }

  func listPluginHooks() async throws -> [RuntimePluginHook] {
    let response: JSONRPCResponse<PluginHookRegistryResult> = try await sendRequest(
      method: "plugin/hookRegistry",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return result.hooks.map { hook in
      RuntimePluginHook(
        hookID: hook.hookId,
        title: hook.title,
        description: hook.description,
        event: hook.event,
        pluginID: hook.pluginId,
        pluginDisplayName: hook.pluginDisplayName,
        permissions: hook.permissions,
        sourcePath: hook.sourcePath,
        memorySummary: hook.memorySummary
      )
    }
  }

  func setPluginEnabled(pluginID: String, enabled: Bool) async throws -> RuntimePlugin {
    let response: JSONRPCResponse<PluginSetEnabledResult> = try await sendRequest(
      method: "plugin/setEnabled",
      params: PluginSetEnabledParams(pluginId: pluginID, enabled: enabled)
    )
    let result = try responseResult(from: response)

    return RuntimePlugin(
      id: result.plugin.id,
      name: result.plugin.name,
      version: result.plugin.version,
      displayName: result.plugin.displayName,
      status: result.plugin.status,
      description: result.plugin.description,
      authorName: result.plugin.authorName,
      enabled: result.plugin.enabled,
      defaultEnabled: result.plugin.defaultEnabled,
      capabilities: result.plugin.capabilities,
      permissions: result.plugin.permissions,
      manifestPath: result.plugin.manifestPath,
      provenance: result.plugin.provenance,
      validationError: result.plugin.validationError,
      validationHint: result.plugin.validationHint
    )
  }

  func removePlugin(manifestPath: String) async throws -> RuntimePluginRemoval {
    let response: JSONRPCResponse<PluginRemoveResult> = try await sendRequest(
      method: "plugin/remove",
      params: PluginRemoveParams(manifestPath: manifestPath)
    )
    let result = try responseResult(from: response)

    return RuntimePluginRemoval(
      pluginID: result.pluginId,
      displayName: result.displayName,
      removedPath: result.removedPath
    )
  }

  func runPluginCommand(threadID: String, commandID: String, input: String? = nil) async throws
    -> RuntimeTurnResult
  {
    let response: JSONRPCResponse<TurnStartResult> = try await sendRequest(
      method: "plugin/commandRun",
      params: PluginCommandRunParams(threadId: threadID, commandId: commandID, input: input)
    )
    let result = try responseResult(from: response)

    return RuntimeTurnResult(
      turnID: result.turnId,
      threadID: result.threadId,
      items: result.items.map(RuntimeBridgePayloadMapper.timelineItem(from:)),
      pendingApprovals: result.pendingApprovals.map(RuntimeBridgePayloadMapper.approval(from:)),
      activeTurnID: result.activeTurnId
    )
  }
}
