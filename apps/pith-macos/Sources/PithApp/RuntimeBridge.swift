import Foundation

final class RuntimeBridge {
  enum ConnectionState: String {
    case disconnected
    case launching
    case ready
    case failed
  }

  struct SessionInfo {
    let serverName: String
    let serverVersion: String
  }

  struct RuntimeThreadSummary {
    let id: String
    let title: String
    let status: String
    let workspaceRootPath: String?
    let workspaceDisplayName: String?
  }

  struct RuntimeWorkspace {
    let rootPath: String
    let displayName: String
    let threadCount: Int
  }

  struct RuntimeWorkspaceSearchMatch {
    let relativePath: String
    let lineNumber: Int
    let line: String
  }

  struct RuntimeModelHealth {
    let packID: String
    let displayName: String
    let backend: String
    let status: String
    let detail: String
    let source: String
    let binaryPath: String?
    let modelPath: String?
    let manifestPath: String?
    let metrics: [String: String]
  }

  struct RuntimeReadinessCheck {
    let id: String
    let title: String
    let status: String
    let detail: String
  }

  struct RuntimeReadiness {
    let status: String
    let summary: String
    let checks: [RuntimeReadinessCheck]
    let metrics: [String: String]
  }

  struct RuntimeModelBootstrap {
    let manifestPath: String
    let readmePath: String?
    let copiedFiles: [String]
  }

  struct RuntimeMemoryStatus {
    let noteCount: Int
    let latestTitle: String?
    let summary: String
  }

  struct RuntimeMemoryNote {
    let id: String
    let title: String
    let body: String
    let scope: String
    let source: String
    let createdAt: Int
    let tags: [String]
  }

  struct RuntimePlugin {
    let id: String
    let name: String
    let version: String
    let displayName: String
    let status: String
    let description: String
    let authorName: String?
    let enabled: Bool
    let defaultEnabled: Bool
    let capabilities: [String]
    let permissions: [String]
    let manifestPath: String
    let provenance: String
    let validationError: String?
    let validationHint: String?
  }

  struct RuntimePluginRemoval {
    let pluginID: String
    let displayName: String
    let removedPath: String
  }

  struct RuntimePluginCapabilityRegistry {
    let capabilities: [RuntimePluginCapability]
    let summary: RuntimePluginCapabilityRegistrySummary
  }

  struct RuntimePluginCapabilityRegistrySummary {
    let enabledPluginCount: Int
    let totalCapabilityCount: Int
    let capabilityCountsByKind: [String: Int]
  }

  struct RuntimePluginCapability {
    let capabilityID: String
    let kind: String
    let identifier: String
    let pluginID: String
    let pluginDisplayName: String
    let permissions: [String]
    let manifestPath: String
    let metadata: [String: String]
  }

  struct RuntimePluginCommand {
    let commandID: String
    let title: String
    let description: String
    let pluginID: String
    let pluginDisplayName: String
    let permissions: [String]
    let sourcePath: String
    let executionKind: String?
    let memorySummary: String?
  }

  struct RuntimePluginConnector {
    let connectorID: String
    let displayName: String
    let service: String
    let pluginID: String
    let pluginDisplayName: String
    let enabled: Bool
    let status: String
    let permissions: [String]
    let manifestPath: String
    let homepage: String?
    let authType: String?
    let authRequired: Bool
    let authScopes: [String]
    let credentialStore: String?
  }

  struct RuntimePluginHook {
    let hookID: String
    let title: String
    let description: String
    let event: String
    let pluginID: String
    let pluginDisplayName: String
    let permissions: [String]
    let sourcePath: String
    let memorySummary: String?
  }

  struct RuntimeTurnResult {
    let turnID: String
    let threadID: String
    let items: [RuntimeTimelineItemResult]
    let pendingApprovals: [RuntimeApproval]
    let activeTurnID: String?
  }

  struct RuntimeThreadState {
    let id: String
    let title: String
    let status: String
    let items: [RuntimeTimelineItemResult]
    let pendingApprovals: [RuntimeApproval]
    let activeTurnID: String?
  }

  struct RuntimeTimelineItemResult {
    let kind: String
    let title: String
    let content: String
    let attributes: [String: String]
  }

  struct RuntimeApproval {
    let id: String
    let threadID: String
    let action: String
    let title: String
    let relativePath: String
  }

  struct RuntimeApprovalResponse {
    let approvalID: String
    let threadID: String
    let items: [RuntimeTimelineItemResult]
    let pendingApprovals: [RuntimeApproval]
  }

  struct RuntimeTurnCancellation {
    let turnID: String
    let threadID: String
    let items: [RuntimeTimelineItemResult]
    let activeTurnID: String?
  }

  enum RuntimeError: LocalizedError {
    case runtimePathMissing
    case runtimePipeUnavailable
    case invalidResponse
    case requestTimedOut(method: String, seconds: Int)
    case rpc(String)

    var errorDescription: String? {
      switch self {
      case .runtimePathMissing:
        return
          "The runtime binary could not be found. " +
          "Set PITH_RUNTIME_PATH to the built runtime executable."
      case .runtimePipeUnavailable:
        return "The runtime process pipes are not available."
      case .invalidResponse:
        return "The runtime returned an invalid response."
      case .requestTimedOut(let method, let seconds):
        return
          "Runtime request \(method) timed out after \(seconds) seconds. " +
          "The local runtime was stopped so it can recover cleanly."
      case .rpc(let message):
        return message
      }
    }
  }

  typealias ThreadUpdatedHandler = @Sendable (RuntimeThreadState) -> Void
  typealias ConnectionStateHandler = @Sendable (ConnectionState, String) -> Void

  private(set) var connectionState: ConnectionState = .disconnected
  var onThreadUpdated: ThreadUpdatedHandler?
  var onConnectionStateChanged: ConnectionStateHandler?

  private var process: Process?
  private var inputHandle: FileHandle?
  private var outputHandle: FileHandle?
  private var nextRequestID: Int = 1
  private let stateQueue = DispatchQueue(label: "pith.runtime.bridge.state")
  private var pendingResponses: [Int: CheckedContinuation<Data, Error>] = [:]
  private var readerTask: Task<Void, Never>?
  private var errorReaderTask: Task<Void, Never>?
  private static let activeModelManifestPathKey = "pith.activeModelManifestPath"
  private static let activeModelPathKey = "pith.activeModelPath"
  private static let defaultRequestTimeoutNanoseconds: UInt64 = 30_000_000_000
  private static let turnRequestTimeoutNanoseconds: UInt64 = 210_000_000_000

  private struct ActiveLocalModelSelection {
    let manifestPath: String
    let modelPath: String
  }

  func launchAndInitialize(launchDetail: String = "Launching local runtime") async throws -> SessionInfo {
    if process == nil || process?.isRunning != true {
      resetProcessState()
      try launchProcess()
    }

    updateConnectionState(.launching, detail: launchDetail)

    let initializeParams = InitializeParams(
      clientInfo: ClientInfo(
        name: "pith-macos",
        version: "0.1.0"
      )
    )

    let response: JSONRPCResponse<InitializeResult> = try await sendRequest(
      method: "initialize",
      params: initializeParams
    )
    let result = try responseResult(from: response)

    updateConnectionState(.ready, detail: "\(result.serverInfo.name) \(result.serverInfo.version)")

    return SessionInfo(
      serverName: result.serverInfo.name,
      serverVersion: result.serverInfo.version
    )
  }

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

  func localPluginInstallRootPath() -> String {
    appSupportPluginDirectory().path
  }

  func localModelStorageRootPath() -> String {
    appSupportStorageDirectory()
      .appendingPathComponent("models", isDirectory: true)
      .path
  }

  func activeLocalModelPath() -> String? {
    activeLocalModelSelection()?.modelPath
  }

  func configureActiveLocalModel(manifestPath: String, modelPath: String) {
    UserDefaults.standard.set(manifestPath, forKey: Self.activeModelManifestPathKey)
    UserDefaults.standard.set(modelPath, forKey: Self.activeModelPathKey)
  }

  func clearActiveLocalModel() {
    UserDefaults.standard.removeObject(forKey: Self.activeModelManifestPathKey)
    UserDefaults.standard.removeObject(forKey: Self.activeModelPathKey)
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

  func stopRuntime(detail: String = "Runtime stopped.") {
    failPendingResponses(with: RuntimeError.rpc(detail))
    if let process, process.isRunning {
      process.terminationHandler = nil
      process.terminate()
    }
    resetProcessState()
    updateConnectionState(.disconnected, detail: detail)
  }

  private func launchProcess() throws {
    let executableURL = try resolveRuntimeURL()
    let process = Process()
    let stdinPipe = Pipe()
    let stdoutPipe = Pipe()
    let stderrPipe = Pipe()

    process.executableURL = executableURL
    process.arguments = []
    process.environment = runtimeEnvironment()
    process.standardInput = stdinPipe
    process.standardOutput = stdoutPipe
    process.standardError = stderrPipe
    let processIdentifier = ObjectIdentifier(process)
    process.terminationHandler = { [weak self] process in
      let detail = "Runtime exited with status \(process.terminationStatus)."
      self?.handleProcessTermination(processIdentifier: processIdentifier, detail: detail)
    }

    try process.run()

    self.process = process
    inputHandle = stdinPipe.fileHandleForWriting
    outputHandle = stdoutPipe.fileHandleForReading
    startReaderLoop(with: stdoutPipe.fileHandleForReading, processIdentifier: processIdentifier)
    startErrorReaderLoop(with: stderrPipe.fileHandleForReading)
  }

  private func startReaderLoop(with handle: FileHandle, processIdentifier: ObjectIdentifier) {
    readerTask?.cancel()
    readerTask = Task.detached(priority: .userInitiated) { [weak self] in
      guard let self else {
        return
      }

      while !Task.isCancelled {
        let line: String
        do {
          line = try Self.readLine(from: handle)
        } catch {
          self.failPendingResponses(with: error)
          self.handleProcessTermination(
            processIdentifier: processIdentifier,
            detail: "Runtime disconnected."
          )
          return
        }

        let data = Data(line.utf8)
        self.handleIncomingMessage(data)
      }
    }
  }

  private func handleIncomingMessage(_ data: Data) {
    let decoder = JSONDecoder()

    if let response = try? decoder.decode(JSONRPCAnyResponse.self, from: data),
       let responseID = response.id
    {
      let continuation = takePendingResponse(requestID: responseID)
      continuation?.resume(returning: data)
      return
    }

    guard let envelope = try? decoder.decode(JSONRPCNotificationEnvelope.self, from: data) else {
      return
    }

    if envelope.method == "thread/updated",
       let notification = try? decoder.decode(
         JSONRPCNotification<ThreadUpdatedNotificationParams>.self,
         from: data
       )
    {
      let state = RuntimeBridgePayloadMapper.threadState(
        id: notification.params.thread.id,
        title: notification.params.thread.title,
        status: notification.params.thread.status,
        items: notification.params.items,
        pendingApprovals: notification.params.pendingApprovals,
        activeTurnID: notification.params.activeTurnId
      )
      onThreadUpdated?(state)
    }
  }

  private func startErrorReaderLoop(with handle: FileHandle) {
    errorReaderTask?.cancel()
    errorReaderTask = Task.detached(priority: .utility) {
      while !Task.isCancelled {
        do {
          let chunk = try handle.read(upToCount: 4096) ?? Data()
          if chunk.isEmpty {
            return
          }

          #if DEBUG
            if let rawMessage = String(data: chunk, encoding: .utf8) {
              let message = rawMessage.trimmingCharacters(in: .whitespacesAndNewlines)
              guard !message.isEmpty else {
                continue
              }
              print("[pith-runtime stderr] \(message)")
            }
          #endif
        } catch {
          return
        }
      }
    }
  }

  private func requestTimeoutNanoseconds(for method: String) -> UInt64 {
    switch method {
    case "turn/start", "plugin/commandRun":
      return Self.turnRequestTimeoutNanoseconds
    default:
      return Self.defaultRequestTimeoutNanoseconds
    }
  }

  private func requestTimeoutSeconds(from timeoutNanoseconds: UInt64) -> Int {
    max(Int(timeoutNanoseconds / 1_000_000_000), 1)
  }

  private func takePendingResponse(requestID: Int) -> CheckedContinuation<Data, Error>? {
    stateQueue.sync {
      pendingResponses.removeValue(forKey: requestID)
    }
  }

  private func handleRequestTimeout(requestID: Int, method: String, timeoutNanoseconds: UInt64) {
    guard let continuation = takePendingResponse(requestID: requestID) else {
      return
    }

    let seconds = requestTimeoutSeconds(from: timeoutNanoseconds)
    let error = RuntimeError.requestTimedOut(method: method, seconds: seconds)
    continuation.resume(throwing: error)
    stopRuntimeAfterRequestTimeout(method: method, seconds: seconds)
  }

  private func handleRequestCancellation(requestID: Int, method: String) {
    guard let continuation = takePendingResponse(requestID: requestID) else {
      return
    }

    let detail = "Runtime request \(method) was cancelled."
    continuation.resume(throwing: RuntimeError.rpc(detail))
    if shouldStopRuntimeAfterCancelledRequest(method: method) {
      stopRuntimeAfterRequestCancellation(method: method)
    }
  }

  private func shouldStopRuntimeAfterCancelledRequest(method: String) -> Bool {
    method == "turn/start" || method == "plugin/commandRun"
  }

  private func stopRuntimeAfterRequestCancellation(method: String) {
    let detail =
      "Runtime request \(method) was cancelled. " +
      "Relaunch the local runtime to continue."
    stopRuntimeAfterRequestBoundary(detail: detail)
  }

  private func stopRuntimeAfterRequestTimeout(method: String, seconds: Int) {
    let detail =
      "Runtime request \(method) timed out after \(seconds) seconds. " +
      "Relaunch the local runtime to continue."
    stopRuntimeAfterRequestBoundary(detail: detail)
  }

  private func stopRuntimeAfterRequestBoundary(detail: String) {
    failPendingResponses(with: RuntimeError.rpc(detail))
    if let process, process.isRunning {
      process.terminationHandler = nil
      process.terminate()
    }
    resetProcessState()
    updateConnectionState(.failed, detail: detail)
  }

  private func failPendingResponses(with error: Error) {
    let continuations = stateQueue.sync {
      let continuations = Array(pendingResponses.values)
      pendingResponses.removeAll()
      return continuations
    }
    for continuation in continuations {
      continuation.resume(throwing: error)
    }
  }

  private func handleProcessTermination(processIdentifier: ObjectIdentifier, detail: String) {
    guard let process, ObjectIdentifier(process) == processIdentifier else {
      return
    }

    failPendingResponses(with: RuntimeError.rpc(detail))
    resetProcessState()
    updateConnectionState(.failed, detail: detail)
  }

  private func resetProcessState() {
    readerTask?.cancel()
    readerTask = nil
    errorReaderTask?.cancel()
    errorReaderTask = nil

    if let process, process.isRunning {
      process.terminationHandler = nil
    }

    process = nil
    inputHandle = nil
    outputHandle = nil
  }

  private func updateConnectionState(_ state: ConnectionState, detail: String) {
    connectionState = state
    onConnectionStateChanged?(state, detail)
  }

  private func resolveRuntimeURL() throws -> URL {
    let environment = ProcessInfo.processInfo.environment

    if let customPath = environment["PITH_RUNTIME_PATH"], !customPath.isEmpty {
      return URL(fileURLWithPath: customPath)
    }

    if let bundledURL = Bundle.main.executableURL?
      .deletingLastPathComponent()
      .appendingPathComponent("pith-runtime-bin"),
      FileManager.default.fileExists(atPath: bundledURL.path)
    {
      return bundledURL
    }

    throw RuntimeError.runtimePathMissing
  }

  private func runtimeEnvironment() -> [String: String] {
    var environment = ProcessInfo.processInfo.environment
    environment["PITH_DATA_DIR"] = appSupportStorageDirectory().path
    environment["PITH_LOCAL_PLUGIN_DIR"] = appSupportPluginDirectory().path
    if let activeModel = activeLocalModelSelection() {
      environment["PITH_MODEL_PACK_MANIFEST"] = activeModel.manifestPath
      environment["PITH_MODEL_PATH"] = activeModel.modelPath
      environment["PITH_LFM_MODEL_PATH"] = activeModel.modelPath
    }
    return environment
  }

  private func activeLocalModelSelection() -> ActiveLocalModelSelection? {
    let defaults = UserDefaults.standard
    guard let manifestPath = defaults.string(forKey: Self.activeModelManifestPathKey),
          !manifestPath.isEmpty,
          let modelPath = defaults.string(forKey: Self.activeModelPathKey),
          !modelPath.isEmpty
    else {
      return nil
    }

    let manager = FileManager.default
    guard manager.fileExists(atPath: manifestPath),
          manager.fileExists(atPath: modelPath)
    else {
      clearActiveLocalModel()
      return nil
    }

    guard LocalModelCatalog.isVerifiedInstalledModel(
      storageRootPath: localModelStorageRootPath(),
      modelPath: modelPath
    ) else {
      clearActiveLocalModel()
      return nil
    }

    return ActiveLocalModelSelection(manifestPath: manifestPath, modelPath: modelPath)
  }

  private func appSupportStorageDirectory() -> URL {
    let baseDirectory =
      FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
      ?? URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)

    return baseDirectory
      .appendingPathComponent("Pith", isDirectory: true)
      .appendingPathComponent("storage", isDirectory: true)
  }

  private func appSupportPluginDirectory() -> URL {
    let baseDirectory =
      FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
      ?? URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)

    return baseDirectory
      .appendingPathComponent("Pith", isDirectory: true)
      .appendingPathComponent("plugins", isDirectory: true)
  }

  private func sendRequest<Params: Encodable, ResultType: Decodable>(
    method: String,
    params: Params
  ) async throws -> JSONRPCResponse<ResultType> {
    guard let inputHandle else {
      throw RuntimeError.runtimePipeUnavailable
    }

    let requestID = stateQueue.sync { () -> Int in
      let id = nextRequestID
      nextRequestID += 1
      return id
    }
    let timeoutNanoseconds = requestTimeoutNanoseconds(for: method)
    let timeoutTask = Task { [weak self] in
      do {
        try await Task.sleep(nanoseconds: timeoutNanoseconds)
      } catch {
        return
      }
      guard !Task.isCancelled else {
        return
      }
      self?.handleRequestTimeout(
        requestID: requestID,
        method: method,
        timeoutNanoseconds: timeoutNanoseconds
      )
    }

    let data: Data
    do {
      data = try await withTaskCancellationHandler(operation: {
        try await withCheckedThrowingContinuation { continuation in
          stateQueue.sync {
            self.pendingResponses[requestID] = continuation
          }

          do {
            let request = JSONRPCRequest(
              id: requestID,
              method: method,
              params: params
            )
            let encoder = JSONEncoder()
            let payload = try encoder.encode(request) + Data([0x0A])
            try inputHandle.write(contentsOf: payload)
          } catch {
            let pending = takePendingResponse(requestID: requestID)
            pending?.resume(throwing: error)
          }
        }
      }, onCancel: {
        timeoutTask.cancel()
        handleRequestCancellation(requestID: requestID, method: method)
      })
    } catch {
      timeoutTask.cancel()
      throw error
    }
    timeoutTask.cancel()

    let decoder = JSONDecoder()
    return try decoder.decode(JSONRPCResponse<ResultType>.self, from: data)
  }

  private func responseResult<ResultType: Decodable>(
    from response: JSONRPCResponse<ResultType>
  ) throws -> ResultType {
    if let error = response.error {
      throw RuntimeError.rpc(error.message)
    }

    guard let result = response.result else {
      throw RuntimeError.invalidResponse
    }

    return result
  }

  private static func readLine(from handle: FileHandle) throws -> String {
    var data = Data()

    while true {
      let chunk = try handle.read(upToCount: 1) ?? Data()

      if chunk.isEmpty {
        break
      }

      if chunk == Data([0x0A]) {
        break
      }

      data.append(chunk)
    }

    guard !data.isEmpty else {
      throw RuntimeError.invalidResponse
    }

    return String(decoding: data, as: UTF8.self)
  }
}
