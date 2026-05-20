import Foundation

extension RuntimeBridge {
  func listPlugins() async throws -> [RuntimePlugin] {
    let response: JSONRPCResponse<PluginListResult> = try await sendRequest(
      method: "plugin/list",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return result.plugins.map { runtimePlugin(from: $0) }
  }

  func refreshPluginCatalog() async throws -> RuntimePluginRefresh {
    let response: JSONRPCResponse<PluginRefreshResult> = try await sendRequest(
      method: "plugin/refresh",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return RuntimePluginRefresh(
      plugins: result.plugins.map { runtimePlugin(from: $0) },
      stateWarning: result.stateWarning
    )
  }

  func installPlugin(sourcePath: String) async throws -> RuntimePlugin {
    let response: JSONRPCResponse<PluginInstallResult> = try await sendRequest(
      method: "plugin/install",
      params: PluginInstallParams(sourcePath: sourcePath)
    )
    let result = try responseResult(from: response)

    return runtimePlugin(from: result.plugin)
  }

  func inspectPlugin(sourcePath: String) async throws -> RuntimePluginInspection {
    let response: JSONRPCResponse<PluginInspectResult> = try await sendRequest(
      method: "plugin/inspect",
      params: PluginInspectParams(sourcePath: sourcePath)
    )
    let result = try responseResult(from: response)

    return RuntimePluginInspection(
      plugin: runtimePlugin(from: result.plugin),
      installStatus: result.installStatus,
      installBlocker: result.installBlocker,
      installRepairHint: result.installRepairHint
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
            input: RuntimePluginCommandEnvelopeMapper.map($0.input),
            output: RuntimePluginCommandEnvelopeMapper.map($0.output),
            supported: $0.supported
          )
        },
        executionKind: command.executionKind,
        memorySummary: command.memorySummary,
        runStatus: command.runStatus,
        runBlocker: command.runBlocker,
        runRepairHint: command.runRepairHint,
        declaredConnectorIds: command.declaredConnectorIds ?? [],
        requiredConnectorIds: command.requiredConnectorIds,
        approvalRequired: command.approvalRequired ?? false,
        approvalReason: command.approvalReason
      )
    }
  }

  func listPluginConnectors() async throws -> [RuntimePluginConnector] {
    let response: JSONRPCResponse<PluginConnectorRegistryResult> = try await sendRequest(
      method: "plugin/connectorRegistry",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return result.connectors.map { runtimePluginConnector(from: $0) }
  }

  func authorizePluginConnector(
    connectorID: String,
    credentialLabel: String? = nil,
    credentialSecret: String? = nil
  ) async throws -> RuntimePluginConnector {
    let response: JSONRPCResponse<PluginConnectorCredentialResult> = try await sendRequest(
      method: "plugin/connectorAuthorize",
      params: PluginConnectorCredentialParams(
        connectorId: connectorID,
        credentialLabel: credentialLabel,
        credentialSecret: credentialSecret
      )
    )
    let result = try responseResult(from: response)

    return runtimePluginConnector(from: result.connector)
  }

  func clearPluginConnectorCredential(connectorID: String) async throws -> RuntimePluginConnector {
    let response: JSONRPCResponse<PluginConnectorCredentialResult> = try await sendRequest(
      method: "plugin/connectorClearCredential",
      params: PluginConnectorCredentialParams(connectorId: connectorID)
    )
    let result = try responseResult(from: response)

    return runtimePluginConnector(from: result.connector)
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
        status: hook.status ?? "ready",
        runBlocker: hook.runBlocker,
        runRepairHint: hook.runRepairHint,
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

    return runtimePlugin(from: result.plugin)
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

private enum RuntimePluginCommandEnvelopeMapper {
  static func map(
    _ payload: RuntimePluginCommandEnvelopePayload?
  ) -> RuntimeBridge.RuntimePluginCommandEnvelope? {
    guard let payload else {
      return nil
    }

    return RuntimeBridge.RuntimePluginCommandEnvelope(
      envelope: payload.envelope,
      fields: payload.fields.map {
        RuntimeBridge.RuntimePluginCommandEnvelopeField(
          name: $0.name,
          kind: $0.kind,
          required: $0.required,
          description: $0.description
        )
      }
    )
  }
}

private extension RuntimeBridge {
  func runtimePlugin(from plugin: RuntimePluginPayload) -> RuntimePlugin {
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

  func runtimePluginConnector(
    from connector: RuntimePluginConnectorPayload
  ) -> RuntimePluginConnector {
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
      credentialStore: connector.credentialStore,
      authStatus: connector.authStatus,
      credentialPresent: connector.credentialPresent,
      credentialSecretPresent: connector.credentialSecretPresent,
      credentialProvider: connector.credentialProvider,
      credentialHandle: connector.credentialHandle,
      credentialLabel: connector.credentialLabel,
      authorizedAt: connector.authorizedAt,
      credentialUpdatedAt: connector.credentialUpdatedAt
    )
  }
}
