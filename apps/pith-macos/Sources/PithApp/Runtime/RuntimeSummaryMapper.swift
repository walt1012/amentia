import Foundation

enum RuntimeSummaryMapper {
  static func threadSummary(
    from runtimeThread: RuntimeBridge.RuntimeThreadSummary
  ) -> ThreadSummary {
    ThreadSummary(
      id: runtimeThread.id,
      title: runtimeThread.title,
      preview: runtimeThread.status,
      workspaceRootPath: runtimeThread.workspaceRootPath,
      workspaceDisplayName: runtimeThread.workspaceDisplayName
    )
  }

  static func modelHealthSummary(
    from runtimeModel: RuntimeBridge.RuntimeModelHealth
  ) -> ModelHealthSummary {
    ModelHealthSummary(
      packID: runtimeModel.packID,
      displayName: runtimeModel.displayName,
      backend: runtimeModel.backend,
      status: runtimeModel.status,
      detail: runtimeModel.detail,
      source: runtimeModel.source,
      binaryPath: runtimeModel.binaryPath,
      modelPath: runtimeModel.modelPath,
      manifestPath: runtimeModel.manifestPath,
      metrics: runtimeModel.metrics
    )
  }

  static func readinessSummary(
    from readiness: RuntimeBridge.RuntimeReadiness
  ) -> RuntimeReadinessSummary {
    RuntimeReadinessSummary(
      status: readiness.status,
      summary: readiness.summary,
      checks: readiness.checks.map { check in
        RuntimeReadinessCheckSummary(
          id: check.id,
          title: check.title,
          status: check.status,
          detail: check.detail
        )
      },
      metrics: readiness.metrics
    )
  }

  static func memoryStatusSummary(
    from status: RuntimeBridge.RuntimeMemoryStatus
  ) -> MemoryStatusSummary {
    MemoryStatusSummary(
      noteCount: status.noteCount,
      latestTitle: status.latestTitle,
      summary: status.summary
    )
  }

  static func memoryNoteSummary(from note: RuntimeBridge.RuntimeMemoryNote) -> MemoryNoteSummary {
    MemoryNoteSummary(
      id: note.id,
      title: note.title,
      body: note.body,
      scope: note.scope,
      source: note.source,
      createdAt: note.createdAt,
      tags: note.tags
    )
  }

  static func pluginSummary(from plugin: RuntimeBridge.RuntimePlugin) -> PluginSummary {
    PluginSummary(
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

  static func pluginRegistrySummary(
    from summary: RuntimeBridge.RuntimePluginCapabilityRegistrySummary
  ) -> PluginCapabilityRegistrySummary {
    PluginCapabilityRegistrySummary(
      enabledPluginCount: summary.enabledPluginCount,
      totalCapabilityCount: summary.totalCapabilityCount,
      capabilityCountsByKind: summary.capabilityCountsByKind
    )
  }

  static func pluginCapabilitySummary(
    from capability: RuntimeBridge.RuntimePluginCapability
  ) -> PluginCapabilitySummary {
    PluginCapabilitySummary(
      id: capability.capabilityID,
      kind: capability.kind,
      identifier: capability.identifier,
      pluginID: capability.pluginID,
      pluginDisplayName: capability.pluginDisplayName,
      permissions: capability.permissions,
      manifestPath: capability.manifestPath,
      metadata: capability.metadata
    )
  }

  static func pluginCommandSummary(
    from command: RuntimeBridge.RuntimePluginCommand
  ) -> PluginCommandSummary {
    PluginCommandSummary(
      id: command.commandID,
      title: command.title,
      description: command.description,
      pluginID: command.pluginID,
      pluginDisplayName: command.pluginDisplayName,
      permissions: command.permissions,
      sourcePath: command.sourcePath,
      execution: command.execution.map {
        PluginCommandExecutionSummary(
          kind: $0.kind,
          driver: $0.driver,
          entrypoint: $0.entrypoint,
          input: pluginCommandEnvelopeSummary(from: $0.input),
          output: pluginCommandEnvelopeSummary(from: $0.output),
          supported: $0.supported
        )
      },
      executionKind: command.executionKind,
      memorySummary: command.memorySummary,
      runStatus: command.runStatus,
      runBlocker: command.runBlocker,
      requiredConnectorIds: command.requiredConnectorIds,
      approvalRequired: command.approvalRequired,
      approvalReason: command.approvalReason
    )
  }

  private static func pluginCommandEnvelopeSummary(
    from envelope: RuntimeBridge.RuntimePluginCommandEnvelope?
  ) -> PluginCommandEnvelopeSummary? {
    guard let envelope else {
      return nil
    }

    return PluginCommandEnvelopeSummary(
      envelope: envelope.envelope,
      fields: envelope.fields.map {
        PluginCommandEnvelopeFieldSummary(
          name: $0.name,
          kind: $0.kind,
          required: $0.required,
          description: $0.description
        )
      }
    )
  }

  static func pluginConnectorSummary(
    from connector: RuntimeBridge.RuntimePluginConnector
  ) -> PluginConnectorSummary {
    PluginConnectorSummary(
      id: connector.connectorID,
      displayName: connector.displayName,
      service: connector.service,
      pluginID: connector.pluginID,
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

  static func pluginHookSummary(from hook: RuntimeBridge.RuntimePluginHook) -> PluginHookSummary {
    PluginHookSummary(
      id: hook.hookID,
      title: hook.title,
      description: hook.description,
      event: hook.event,
      pluginID: hook.pluginID,
      pluginDisplayName: hook.pluginDisplayName,
      permissions: hook.permissions,
      sourcePath: hook.sourcePath,
      memorySummary: hook.memorySummary
    )
  }
}
