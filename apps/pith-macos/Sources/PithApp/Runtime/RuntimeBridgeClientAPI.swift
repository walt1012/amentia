import Foundation

extension RuntimeBridge {
  func listThreads() async throws -> [RuntimeThreadSummary] {
    let response: JSONRPCResponse<ThreadListResult> = try await sendRequest(
      method: "thread/list",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return result.threads.map {
      RuntimeThreadSummary(
        id: $0.id,
        title: $0.title,
        status: $0.status,
        workspaceRootPath: $0.workspace?.rootPath,
        workspaceDisplayName: $0.workspace?.displayName
      )
    }
  }

  func modelHealth() async throws -> RuntimeModelHealth {
    let response: JSONRPCResponse<ModelHealthResult> = try await sendRequest(
      method: "model/health",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return RuntimeModelHealth(
      packID: result.packId,
      displayName: result.displayName,
      backend: result.backend,
      status: result.status,
      detail: result.detail,
      source: result.source,
      binaryPath: result.binaryPath,
      modelPath: result.modelPath,
      manifestPath: result.manifestPath,
      metrics: result.metrics
    )
  }

  func runtimeReadiness() async throws -> RuntimeReadiness {
    let response: JSONRPCResponse<RuntimeReadinessResult> = try await sendRequest(
      method: "runtime/readiness",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return RuntimeReadiness(
      status: result.status,
      summary: result.summary,
      checks: result.checks.map { check in
        RuntimeReadinessCheck(
          id: check.id,
          title: check.title,
          status: check.status,
          detail: check.detail
        )
      },
      metrics: result.metrics
    )
  }

  func bootstrapModelPack() async throws -> RuntimeModelBootstrap {
    let response: JSONRPCResponse<ModelBootstrapResult> = try await sendRequest(
      method: "model/bootstrap",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return RuntimeModelBootstrap(
      manifestPath: result.manifestPath,
      readmePath: result.readmePath,
      copiedFiles: result.copiedFiles
    )
  }

  func memoryStatus() async throws -> RuntimeMemoryStatus {
    let response: JSONRPCResponse<MemoryStatusResult> = try await sendRequest(
      method: "memory/status",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return RuntimeMemoryStatus(
      noteCount: result.noteCount,
      latestTitle: result.latestTitle,
      summary: result.summary
    )
  }

  func listMemoryNotes() async throws -> [RuntimeMemoryNote] {
    let response: JSONRPCResponse<MemoryListResult> = try await sendRequest(
      method: "memory/list",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return result.notes.map { note in
      RuntimeMemoryNote(
        id: note.id,
        title: note.title,
        body: note.body,
        scope: note.scope,
        source: note.source,
        createdAt: note.createdAt,
        tags: note.tags
      )
    }
  }

  func createMemoryNote(title: String, body: String) async throws -> RuntimeMemoryNote {
    let response: JSONRPCResponse<MemoryCreateResult> = try await sendRequest(
      method: "memory/create",
      params: MemoryCreateParams(title: title, body: body)
    )
    let result = try responseResult(from: response)

    return RuntimeMemoryNote(
      id: result.note.id,
      title: result.note.title,
      body: result.note.body,
      scope: result.note.scope,
      source: result.note.source,
      createdAt: result.note.createdAt,
      tags: result.note.tags
    )
  }

  func openWorkspace(path: String) async throws -> RuntimeWorkspace {
    let response: JSONRPCResponse<WorkspaceOpenResult> = try await sendRequest(
      method: "workspace/open",
      params: WorkspaceOpenParams(path: path)
    )
    let result = try responseResult(from: response)

    return RuntimeWorkspace(
      rootPath: result.workspace.rootPath,
      displayName: result.workspace.displayName,
      threadCount: result.threadCount
    )
  }

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

  func currentWorkspace() async throws -> RuntimeWorkspace? {
    let response: JSONRPCResponse<WorkspaceCurrentResult> = try await sendRequest(
      method: "workspace/current",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    guard let workspace = result.workspace else {
      return nil
    }

    return RuntimeWorkspace(
      rootPath: workspace.rootPath,
      displayName: workspace.displayName,
      threadCount: 0
    )
  }

  func searchWorkspace(query: String, maxResults: Int = 24) async throws -> [RuntimeWorkspaceSearchMatch] {
    let response: JSONRPCResponse<WorkspaceSearchResult> = try await sendRequest(
      method: "workspace/search",
      params: WorkspaceSearchParams(query: query, maxResults: maxResults)
    )
    let result = try responseResult(from: response)

    return result.matches.map { match in
      RuntimeWorkspaceSearchMatch(
        relativePath: match.relativePath,
        lineNumber: match.lineNumber,
        line: match.line
      )
    }
  }

  func startThread(title: String) async throws -> ThreadSummary {
    let response: JSONRPCResponse<ThreadStartResult> = try await sendRequest(
      method: "thread/start",
      params: ThreadStartParams(title: title)
    )
    let result = try responseResult(from: response)

    return ThreadSummary(
      id: result.thread.id,
      title: result.thread.title,
      preview: result.thread.status,
      workspaceRootPath: result.thread.workspace?.rootPath,
      workspaceDisplayName: result.thread.workspace?.displayName
    )
  }

  func startTurn(threadID: String, message: String) async throws -> RuntimeTurnResult {
    let response: JSONRPCResponse<TurnStartResult> = try await sendRequest(
      method: "turn/start",
      params: TurnStartParams(threadId: threadID, message: message)
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

  func readThread(threadID: String) async throws -> RuntimeThreadState {
    let response: JSONRPCResponse<ThreadReadResult> = try await sendRequest(
      method: "thread/read",
      params: ThreadReadParams(threadId: threadID)
    )
    let result = try responseResult(from: response)

    return RuntimeBridgePayloadMapper.threadState(
      id: result.thread.id,
      title: result.thread.title,
      status: result.thread.status,
      items: result.items,
      pendingApprovals: result.pendingApprovals,
      activeTurnID: result.activeTurnId
    )
  }

  func respondToApproval(approvalID: String, decision: String) async throws -> RuntimeApprovalResponse {
    let response: JSONRPCResponse<ApprovalRespondResult> = try await sendRequest(
      method: "approval/respond",
      params: ApprovalRespondParams(approvalId: approvalID, decision: decision)
    )
    let result = try responseResult(from: response)

    return RuntimeApprovalResponse(
      approvalID: result.approvalId,
      threadID: result.threadId,
      items: result.items.map(RuntimeBridgePayloadMapper.timelineItem(from:)),
      pendingApprovals: result.pendingApprovals.map(RuntimeBridgePayloadMapper.approval(from:))
    )
  }

  func cancelTurn(turnID: String) async throws -> RuntimeTurnCancellation {
    let response: JSONRPCResponse<TurnCancelResult> = try await sendRequest(
      method: "turn/cancel",
      params: TurnCancelParams(turnId: turnID)
    )
    let result = try responseResult(from: response)

    return RuntimeTurnCancellation(
      turnID: result.turnId,
      threadID: result.threadId,
      items: result.items.map(RuntimeBridgePayloadMapper.timelineItem(from:)),
      activeTurnID: result.activeTurnId
    )
  }
}
