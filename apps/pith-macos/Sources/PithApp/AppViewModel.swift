import AppKit
import Foundation

private struct ModelDownloadPaused: Error {
  let resumeData: Data
}

private struct ModelDownloadProgress: Hashable {
  let modelID: String
  let displayName: String
  var bytesReceived: Int64
  var totalBytes: Int64
  let startedAt: Date
  var updatedAt: Date
  let isResuming: Bool
}

@MainActor
final class AppViewModel: ObservableObject {
  private static let lastWorkspacePathKey = "pith.lastWorkspacePath"
  private let setupStepCount = 4

  private struct LocalPluginAuthor: Decodable {
    let name: String
  }

  private struct LocalPluginManifest: Decodable {
    let name: String
    let version: String
    let displayName: String
    let description: String
    let author: LocalPluginAuthor?
    let capabilities: [String]
    let permissions: [String]
    let defaultEnabled: Bool
  }

  private struct PluginInstallPreview {
    let sourcePath: String
    let manifestPath: String
    let installPath: String
    let displayName: String
    let version: String
    let description: String
    let authorName: String?
    let capabilities: [String]
    let permissions: [String]
    let defaultEnabled: Bool
  }

  private struct LocalModelCatalogItem {
    let id: String
    let displayName: String
    let description: String
    let fileName: String
    let downloadURL: String
    let homepage: String
    let sizeBytes: Int64
    let contextSize: Int
    let maxOutputTokens: Int
    let license: String
    let tags: [String]
    let installSegments: [String]

    func installPath(storageRootPath: String) -> String {
      installSegments.reduce(URL(fileURLWithPath: storageRootPath, isDirectory: true)) { url, segment in
        url.appendingPathComponent(segment)
      }
      .appendingPathComponent(fileName)
      .path
    }
  }

  private struct LocalModelPackManifest: Encodable {
    let id: String
    let displayName: String
    let fileName: String
    let contextSize: Int
    let maxOutputTokens: Int
    let backend: String
    let license: String
    let homepage: String
    let downloadURL: String
    let sizeBytes: Int64

    enum CodingKeys: String, CodingKey {
      case id
      case displayName = "display_name"
      case fileName = "file_name"
      case contextSize = "context_size"
      case maxOutputTokens = "max_output_tokens"
      case backend
      case license
      case homepage
      case downloadURL = "download_url"
      case sizeBytes = "size_bytes"
    }
  }

  @Published var threads: [ThreadSummary]
  @Published var selectedThreadID: ThreadSummary.ID?
  @Published var timeline: [TimelineEntry]
  @Published var selectedEntryID: TimelineEntry.ID?
  @Published var activeTurnID: String?
  @Published var runtimeState: RuntimeBridge.ConnectionState
  @Published var runtimeDetail: String
  @Published var draftMessage: String
  @Published var workspace: WorkspaceSummary?
  @Published var workspaceSearchQuery: String
  @Published var workspaceSearchResults: [WorkspaceSearchMatchSummary]
  @Published var workspaceSearchStatus: String
  @Published var isWorkspaceSearching: Bool
  @Published var modelHealth: ModelHealthSummary?
  @Published var localModels: [LocalModelSummary]
  @Published var modelDownloadID: String?
  @Published var pausedModelDownloadID: String?
  @Published private var modelDownloadProgress: ModelDownloadProgress?
  @Published var memoryStatus: MemoryStatusSummary?
  @Published var memoryNotes: [MemoryNoteSummary]
  @Published var memoryNoteTitle: String
  @Published var memoryNoteBody: String
  @Published var plugins: [PluginSummary]
  @Published var pluginCapabilityRegistrySummary: PluginCapabilityRegistrySummary?
  @Published var pluginCapabilities: [PluginCapabilitySummary]
  @Published var pluginConnectors: [PluginConnectorSummary]
  @Published var pluginCommands: [PluginCommandSummary]
  @Published var pluginHooks: [PluginHookSummary]

  private let runtimeBridge: RuntimeBridge
  private var threadTimelines: [String: [TimelineEntry]]
  private var threadPendingApprovalIDs: [String: Set<String>]
  private var activeTurnThreadID: String?
  private var lastRuntimeFailureDetail: String?
  private var modelDownloadTask: Task<Void, Never>?
  private var modelDownloadTransfer: ModelDownloadTransfer?
  private var modelDownloadResumeData: Data?
  private var announcedSetupCompleteThreadIDs: Set<String>

  init(runtimeBridge: RuntimeBridge = RuntimeBridge()) {
    let initialTimeline = [
      TimelineEntry(
        id: UUID().uuidString,
        kind: .system,
        title: "Start Local Setup",
        body: "Launch the runtime, download the local LFM2.5-350M model, open a workspace, then create or select a thread.",
        attributes: [
          "path": "runtime -> model -> workspace -> thread"
        ]
      ),
      TimelineEntry(
        id: UUID().uuidString,
        kind: .assistantMessage,
        title: "Local-First Agent Loop",
        body:
          "Pith runs the core agent loop against local workspaces and does not use an external model API fallback.",
        attributes: [
          "model": "local"
        ]
      ),
    ]

    let initialThreads = [
      ThreadSummary(
        id: "local-welcome",
        title: "Welcome to Pith",
        preview: "Open a workspace to begin the local agent loop."
      ),
    ]

    self.runtimeBridge = runtimeBridge
    self.runtimeState = runtimeBridge.connectionState
    self.runtimeDetail = "Runtime not launched"
    self.draftMessage = ""
    self.workspace = nil
    self.workspaceSearchQuery = ""
    self.workspaceSearchResults = []
    self.workspaceSearchStatus = "Search the open workspace by text."
    self.isWorkspaceSearching = false
    self.modelHealth = nil
    self.localModels = Self.localModelSummaries(
      storageRootPath: runtimeBridge.localModelStorageRootPath(),
      activeModelPath: runtimeBridge.activeLocalModelPath()
    )
    self.modelDownloadID = nil
    self.pausedModelDownloadID = nil
    self.modelDownloadProgress = nil
    self.memoryStatus = nil
    self.memoryNotes = []
    self.memoryNoteTitle = ""
    self.memoryNoteBody = ""
    self.plugins = []
    self.pluginCapabilityRegistrySummary = nil
    self.pluginCapabilities = []
    self.pluginConnectors = []
    self.pluginCommands = []
    self.pluginHooks = []
    self.threads = initialThreads
    self.timeline = initialTimeline
    self.selectedEntryID = initialTimeline.first?.id
    self.activeTurnID = nil
    self.threadTimelines = ["local-welcome": initialTimeline]
    self.threadPendingApprovalIDs = [:]
    self.lastRuntimeFailureDetail = nil
    self.modelDownloadTask = nil
    self.modelDownloadTransfer = nil
    self.modelDownloadResumeData = nil
    self.announcedSetupCompleteThreadIDs = Set<String>()
    self.selectedThreadID = initialThreads.first?.id
    self.runtimeBridge.onThreadUpdated = { [weak self] state in
      Task { @MainActor in
        self?.applyRuntimeThreadUpdate(state)
      }
    }
    self.runtimeBridge.onConnectionStateChanged = { [weak self] state, detail in
      Task { @MainActor in
        self?.handleRuntimeConnectionStateChange(state, detail: detail)
      }
    }
  }

  func launchRuntime() {
    guard runtimeState != .launching else {
      return
    }

    if runtimeState == .ready {
      runtimeBridge.stopRuntime(detail: "Relaunching local runtime...")
    }

    runtimeState = .launching
    runtimeDetail = "Launching local runtime"
    lastRuntimeFailureDetail = nil

    Task {
      do {
        let session = try await runtimeBridge.launchAndInitialize()
        let runtimeMemoryStatus = try? await runtimeBridge.memoryStatus()
        let runtimeMemoryNotes = try? await runtimeBridge.listMemoryNotes()
        var currentWorkspace = try? await runtimeBridge.currentWorkspace()
        var restoredWorkspace = false
        var workspaceRestoreError: Error?
        if currentWorkspace == nil,
           let lastWorkspacePath = storedLastWorkspacePath(),
           isRestorableWorkspacePath(lastWorkspacePath)
        {
          do {
            currentWorkspace = try await runtimeBridge.openWorkspace(path: lastWorkspacePath)
            restoredWorkspace = true
          } catch {
            workspaceRestoreError = error
          }
        }
        let threadList = try await runtimeBridge.listThreads()

        runtimeState = .ready
        await refreshModelHealthState(serverLabel: "\(session.serverName) \(session.serverVersion)")

        if let runtimeMemoryStatus {
          memoryStatus = MemoryStatusSummary(
            noteCount: runtimeMemoryStatus.noteCount,
            latestTitle: runtimeMemoryStatus.latestTitle,
            summary: runtimeMemoryStatus.summary
          )
        } else {
          memoryStatus = nil
        }
        memoryNotes = (runtimeMemoryNotes ?? []).map { note in
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

        await refreshPluginState()
        if !plugins.isEmpty {
          runtimeDetail += " | \(plugins.count) plugin(s)"
        }
        if !pluginCapabilities.isEmpty {
          runtimeDetail += " | \(pluginCapabilities.count) capability(s)"
        }
        if !pluginCommands.isEmpty {
          runtimeDetail += " | \(pluginCommands.count) command(s)"
        }
        if !pluginConnectors.isEmpty {
          runtimeDetail += " | \(pluginConnectors.count) connector(s)"
        }
        if !pluginHooks.isEmpty {
          runtimeDetail += " | \(pluginHooks.count) hook(s)"
        }

        if let currentWorkspace {
          workspace = WorkspaceSummary(
            rootPath: currentWorkspace.rootPath,
            displayName: currentWorkspace.displayName
          )
          resetWorkspaceSearch()
          storeLastWorkspacePath(currentWorkspace.rootPath)
        }

        if threadList.isEmpty {
          let firstThread = try await runtimeBridge.startThread(title: "Workspace Thread")
          threads = [firstThread]
          threadTimelines = [firstThread.id: defaultTimeline(for: firstThread.title)]
        } else {
          threads = threadList.map { ThreadSummary(id: $0.id, title: $0.title, preview: $0.status) }
          threadTimelines = Dictionary(
            uniqueKeysWithValues: threads.map { thread in
              (thread.id, defaultTimeline(for: thread.title))
            }
          )
        }

        let selectedThread = threads.first
        selectThread(id: selectedThread?.id)
        if let selectedThreadID = selectedThread?.id {
          await loadThreadHistory(threadID: selectedThreadID)
        }
        appendEntry(
          to: selectedThread?.id,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: "Runtime Connected",
            body: "Connected to \(session.serverName) \(session.serverVersion) over stdio.",
            attributes: [:]
          )
        )
        if restoredWorkspace, let currentWorkspace {
          appendEntry(
            to: selectedThread?.id,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .system,
              title: "Workspace Restored",
              body: "Restored \(currentWorkspace.displayName) at \(currentWorkspace.rootPath).",
              attributes: [
                "workspacePath": currentWorkspace.rootPath
              ]
            )
          )
        }
        if let workspaceRestoreError {
          appendEntry(
            to: selectedThread?.id,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .warning,
              title: "Workspace Restore Failed",
              body: workspaceRestoreError.localizedDescription,
              attributes: [:]
            )
          )
        }
        if let runtimeModel = modelHealth {
          if isLocalModelReady() {
            appendEntry(
              to: selectedThread?.id,
              TimelineEntry(
                id: UUID().uuidString,
                kind: .system,
                title: "Local Model Ready",
                body:
                  "\(runtimeModel.displayName) is running in \(runtimeModel.backend) mode with status \(runtimeModel.status).",
                attributes: [
                  "modelId": runtimeModel.packID,
                  "modelBackend": runtimeModel.backend,
                  "modelStatus": runtimeModel.status,
                  "modelSource": runtimeModel.source,
                ]
              )
            )
          } else {
            appendEntry(
              to: selectedThread?.id,
              TimelineEntry(
                id: UUID().uuidString,
                kind: .warning,
                title: "Local Model Required",
                body: localModelRequiredTimelineSummary(),
                attributes: [
                  "modelId": runtimeModel.packID,
                  "modelBackend": runtimeModel.backend,
                  "modelStatus": runtimeModel.status,
                  "modelSource": runtimeModel.source,
                ]
              )
            )
          }
        } else {
          appendEntry(
            to: selectedThread?.id,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .warning,
              title: "Local Model Required",
              body: localModelRequiredTimelineSummary(),
              attributes: [
                "modelStatus": "unavailable"
              ]
            )
          )
        }
        if let runtimeMemoryStatus {
          appendEntry(
            to: selectedThread?.id,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .system,
              title: "Memory Ready",
              body: runtimeMemoryStatus.summary,
              attributes: [
                "noteCount": String(runtimeMemoryStatus.noteCount)
              ]
            )
          )
        }
        if !plugins.isEmpty {
          appendEntry(
            to: selectedThread?.id,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .system,
              title: "Plugins Ready",
              body: "Discovered \(plugins.count) plugin(s): \(plugins.map(\.displayName).joined(separator: ", ")).",
              attributes: [:]
            )
          )
        }
        if let registrySummary = pluginCapabilityRegistrySummary,
           registrySummary.totalCapabilityCount > 0 {
          appendEntry(
            to: selectedThread?.id,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .system,
              title: "Capability Registry Ready",
              body:
                "Registered \(registrySummary.totalCapabilityCount) capability(ies) across \(registrySummary.enabledPluginCount) enabled plugin(s).",
              attributes: [
                "enabledPluginCount": String(registrySummary.enabledPluginCount),
                "totalCapabilityCount": String(registrySummary.totalCapabilityCount)
              ]
            )
          )
        }
        if !pluginCommands.isEmpty {
          appendEntry(
            to: selectedThread?.id,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .system,
              title: "Plugin Commands Ready",
              body:
                "Loaded \(pluginCommands.count) plugin command(s): \(pluginCommands.map(\.title).joined(separator: ", ")).",
              attributes: [:]
            )
          )
        }
        if !pluginHooks.isEmpty {
          appendEntry(
            to: selectedThread?.id,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .system,
              title: "Plugin Hooks Ready",
              body:
                "Loaded \(pluginHooks.count) plugin hook(s): \(pluginHooks.map(\.title).joined(separator: ", ")).",
              attributes: [:]
            )
          )
        }
        announceSetupCompleteIfNeeded()
      } catch {
        runtimeState = .failed
        runtimeDetail = error.localizedDescription
        modelHealth = nil
        memoryStatus = nil
        memoryNotes = []
        plugins = []
        pluginCapabilityRegistrySummary = nil
        pluginCapabilities = []
        pluginConnectors = []
        pluginCommands = []
        pluginHooks = []
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Runtime Launch Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
        )
      }
    }
  }

  func runtimeLaunchButtonTitle() -> String {
    switch runtimeState {
    case .ready:
      return "Relaunch Runtime"
    case .failed:
      return "Relaunch Runtime"
    case .launching:
      return "Launching Runtime"
    case .disconnected:
      return "Launch Runtime"
    }
  }

  func runtimeStatusSummary() -> String {
    switch runtimeState {
    case .disconnected:
      return "Launch the local runtime to restore model, workspace, plugins, and memory."
    case .launching:
      return "Starting the local runtime and reconnecting app state..."
    case .failed:
      return "Runtime stopped. Relaunch to recover the local agent loop."
    case .ready:
      if !isLocalModelReady() {
        if modelDownloadID != nil {
          return "Downloading the local model. Agent work unlocks when it is ready."
        }
        if pausedModelDownloadID != nil {
          return "Model download is paused. Continue it from Local Model."
        }
        if isDefaultModelDownloaded() {
          return "Use the downloaded default model to complete the local setup."
        }
        return "Download the local LFM2.5-350M model to enable offline agent work."
      }
      if workspace == nil {
        return "Model is ready. Open a workspace to bind tools to a project."
      }
      if activeTurnID != nil {
        return "Pith is streaming locally. Cancel only if the turn is no longer useful."
      }
      if !hasRuntimeThreadSelection() {
        return "Select or create a thread to start local agent work."
      }
      return "Ready for local agent work."
    }
  }

  func runtimeStatusTone() -> StatusTone {
    switch runtimeState {
    case .disconnected:
      return .warning
    case .launching:
      return .active
    case .failed:
      return .danger
    case .ready:
      if activeTurnID != nil || modelDownloadID != nil || isWorkspaceSearching {
        return .active
      }
      if workspace == nil || !isLocalModelReady() || !hasRuntimeThreadSelection() {
        return .warning
      }
      return .ready
    }
  }

  func showsRuntimeActivity() -> Bool {
    runtimeState == .launching
      || isWorkspaceSearching
      || modelDownloadID != nil
      || activeTurnID != nil
  }

  func runtimeReadinessSteps() -> [ReadinessStepSummary] {
    [
      runtimeReadinessStep(),
      modelReadinessStep(),
      workspaceReadinessStep(),
      threadReadinessStep(),
    ]
  }

  func setupProgressSummary() -> String {
    let readyCount = setupReadyStepCount()
    if readyCount == setupStepCount {
      return "Local setup complete"
    }

    return "Local setup \(readyCount)/\(setupStepCount)"
  }

  func setupProgressValue() -> Double {
    Double(setupReadyStepCount()) / Double(setupStepCount)
  }

  func setupProgressTone() -> StatusTone {
    if runtimeState == .failed {
      return .danger
    }
    if showsRuntimeActivity() {
      return .active
    }
    return setupReadyStepCount() == setupStepCount ? .ready : .warning
  }

  func runtimePrimaryActionTitle() -> String? {
    switch runtimeState {
    case .disconnected, .failed, .launching:
      return runtimeLaunchButtonTitle()
    case .ready:
      if !isLocalModelReady() {
        if modelDownloadID != nil {
          return nil
        }
        if canDownloadLocalModel() {
          return defaultModelDownloadButtonTitle()
        }
        if canBootstrapModelPackMetadata() {
          return "Install Metadata"
        }
        return nil
      }
      if workspace == nil {
        return "Open Workspace"
      }
      if activeTurnID != nil {
        return "Cancel Turn"
      }
      if !hasRuntimeThreadSelection() {
        return "New Thread"
      }
      return nil
    }
  }

  func canRunRuntimePrimaryAction() -> Bool {
    switch runtimeState {
    case .disconnected, .failed:
      return canLaunchRuntime()
    case .launching:
      return false
    case .ready:
      if !isLocalModelReady() {
        return modelDownloadID == nil
          && (canDownloadLocalModel() || canBootstrapModelPackMetadata())
      }
      if workspace == nil {
        return canOpenWorkspace()
      }
      if activeTurnID != nil {
        return canCancelActiveTurn()
      }
      if !hasRuntimeThreadSelection() {
        return canCreateThread()
      }
      return false
    }
  }

  func runRuntimePrimaryAction() {
    switch runtimeState {
    case .disconnected, .failed:
      launchRuntime()
    case .launching:
      return
    case .ready:
      if !isLocalModelReady() {
        if canDownloadLocalModel() {
          downloadLocalModel()
        } else if canBootstrapModelPackMetadata() {
          bootstrapModelPackMetadata()
        }
        return
      }
      if workspace == nil {
        openWorkspace()
        return
      }
      if activeTurnID != nil {
        cancelActiveTurn()
        return
      }
      if !hasRuntimeThreadSelection() {
        createThread()
      }
    }
  }

  func canLaunchRuntime() -> Bool {
    runtimeState != .launching
  }

  func canOpenWorkspace() -> Bool {
    runtimeState == .ready
  }

  func canCreateThread() -> Bool {
    runtimeState == .ready
  }

  func canInstallPlugin() -> Bool {
    runtimeState == .ready
  }

  func canSendDraftMessage() -> Bool {
    runtimeState == .ready
      && isLocalModelReady()
      && hasRuntimeThreadSelection()
      && !isTurnStreaming()
      && !draftMessage.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
  }

  func canCancelActiveTurn() -> Bool {
    runtimeState == .ready && isTurnStreaming()
  }

  func canUseComposer() -> Bool {
    runtimeState == .ready
      && workspace != nil
      && isLocalModelReady()
      && hasRuntimeThreadSelection()
      && activeTurnID == nil
  }

  func canSearchWorkspace() -> Bool {
    runtimeState == .ready
      && workspace != nil
      && !isWorkspaceSearching
      && !workspaceSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
  }

  func searchWorkspace() {
    let query = workspaceSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines)
    guard runtimeState == .ready, workspace != nil, !query.isEmpty else {
      return
    }

    isWorkspaceSearching = true
    workspaceSearchStatus = "Searching for \"\(query)\"..."
    Task {
      do {
        let matches = try await runtimeBridge.searchWorkspace(query: query)
        workspaceSearchResults = matches.enumerated().map { index, match in
          WorkspaceSearchMatchSummary(
            id: "\(match.relativePath):\(match.lineNumber):\(index)",
            relativePath: match.relativePath,
            lineNumber: match.lineNumber,
            line: match.line
          )
        }
        workspaceSearchStatus = matches.isEmpty
          ? "No matches found for \"\(query)\"."
          : "Found \(matches.count) match(es) for \"\(query)\"."
      } catch {
        workspaceSearchResults = []
        workspaceSearchStatus = "Workspace search failed: \(error.localizedDescription)"
      }
      isWorkspaceSearching = false
    }
  }

  func clearWorkspaceSearch() {
    workspaceSearchQuery = ""
    resetWorkspaceSearch()
  }

  func workspaceSearchEmptyStateSummary() -> String? {
    if isWorkspaceSearching || !workspaceSearchResults.isEmpty {
      return nil
    }
    if runtimeState != .ready {
      return "Launch the runtime to search workspace files."
    }
    if workspace == nil {
      return "Open a workspace to search local files."
    }
    if workspaceSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
      return "Search file contents or symbols, then press Return."
    }
    if workspaceSearchStatus.hasPrefix("No matches found") {
      return "No results yet. Try a shorter query, filename, or symbol name."
    }
    if workspaceSearchStatus.hasPrefix("Workspace search failed") {
      return "Search failed. Check the runtime status, then try again."
    }
    return nil
  }

  func workspaceSearchOverflowSummary() -> String? {
    guard workspaceSearchResults.count > 8 else {
      return nil
    }

    return "Showing the first 8 matches. Narrow the query to focus the review."
  }

  func openWorkspace() {
    guard runtimeState == .ready else {
      return
    }

    let panel = NSOpenPanel()
    panel.canChooseDirectories = true
    panel.canChooseFiles = false
    panel.allowsMultipleSelection = false
    panel.prompt = "Open Workspace"
    panel.message = "Choose a local workspace for Pith to inspect."

    guard panel.runModal() == .OK, let url = panel.url else {
      return
    }

    Task {
      do {
        let openedWorkspace = try await runtimeBridge.openWorkspace(path: url.path)
        workspace = WorkspaceSummary(
          rootPath: openedWorkspace.rootPath,
          displayName: openedWorkspace.displayName
        )
        resetWorkspaceSearch()
        storeLastWorkspacePath(openedWorkspace.rootPath)
        await refreshMemoryState()
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: "Workspace Opened",
            body: "Opened \(openedWorkspace.displayName) at \(openedWorkspace.rootPath).",
            attributes: [:]
          )
        )
        announceSetupCompleteIfNeeded()
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Workspace Open Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
        )
      }
    }
  }

  func installPlugin() {
    guard runtimeState == .ready else {
      return
    }

    let panel = NSOpenPanel()
    panel.canChooseDirectories = true
    panel.canChooseFiles = true
    panel.allowsMultipleSelection = false
    panel.prompt = "Install Plugin"
    panel.message = "Choose a plugin folder or a pith-plugin.json manifest."

    guard panel.runModal() == .OK, let url = panel.url else {
      return
    }

    let preview: PluginInstallPreview
    do {
      preview = try inspectPluginInstallCandidate(at: url)
    } catch {
      let repairHint = pluginInstallRepairHint(for: error)
      let body = repairHint.isEmpty ? error.localizedDescription : "\(error.localizedDescription)\n\nRepair Hint: \(repairHint)"
      appendEntry(
        to: selectedThreadID,
        TimelineEntry(
          id: UUID().uuidString,
          kind: .warning,
          title: "Plugin Install Preview Failed",
          body: body,
          attributes: [:]
        )
      )
      return
    }

    guard confirmPluginInstall(preview: preview) else {
      runtimeDetail = "Plugin install was cancelled."
      return
    }

    Task {
      do {
        let installedPlugin = try await runtimeBridge.installPlugin(sourcePath: preview.sourcePath)
        await refreshPluginState()
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: "Plugin Installed",
            body:
              "\(installedPlugin.displayName) is now available in the local plugin manager.\nSource: \(preview.sourcePath)\nInstalled To: \(preview.installPath)",
            attributes: [
              "pluginId": installedPlugin.id,
              "pluginStatus": installedPlugin.status,
              "pluginManifestPath": installedPlugin.manifestPath,
              "pluginSourcePath": preview.sourcePath,
              "pluginInstallPath": preview.installPath,
            ]
          )
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Plugin Install Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
        )
      }
    }
  }

  func createThread() {
    guard runtimeState == .ready else {
      return
    }

    Task {
      do {
        let thread = try await runtimeBridge.startThread(title: "Thread \(threads.count + 1)")
        threads.insert(thread, at: 0)
        threadTimelines[thread.id] = defaultTimeline(for: thread.title)
        threadPendingApprovalIDs[thread.id] = Set<String>()
        selectThread(id: thread.id)
        await loadThreadHistory(threadID: thread.id)
        appendEntry(
          to: thread.id,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: "Thread Created",
            body: "Created \(thread.title) in the local runtime.",
            attributes: [:]
          )
        )
        announceSetupCompleteIfNeeded()
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Thread Creation Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
        )
      }
    }
  }

  func sendDraftMessage() {
    let message = draftMessage.trimmingCharacters(in: .whitespacesAndNewlines)

    guard runtimeState == .ready,
          workspace != nil,
          isLocalModelReady(),
          !message.isEmpty,
          let threadID = selectedThreadID,
          !threadID.hasPrefix("local-"),
          activeTurnID == nil
    else {
      return
    }

    draftMessage = ""

    Task {
      do {
        let result = try await runtimeBridge.startTurn(threadID: threadID, message: message)
        appendItemsToTimeline(threadID: result.threadID, items: result.items)
        updatePendingApprovals(threadID: result.threadID, approvals: result.pendingApprovals)
        updateActiveTurn(threadID: result.threadID, activeTurnID: result.activeTurnID)
        refreshThreadPreview(
          threadID: result.threadID,
          preview: result.activeTurnID == nil ? "\(result.turnID) ready" : "Streaming response"
        )
      } catch {
        appendEntry(
          to: threadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Turn Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
        )
      }
    }
  }

  func respondToApproval(approvalID: String, decision: String) {
    guard runtimeState == .ready else {
      return
    }

    Task {
      do {
        let result = try await runtimeBridge.respondToApproval(
          approvalID: approvalID,
          decision: decision
        )
        appendItemsToTimeline(threadID: result.threadID, items: result.items)
        updatePendingApprovals(threadID: result.threadID, approvals: result.pendingApprovals)
        await refreshMemoryState()
        await loadThreadHistory(threadID: result.threadID)
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Approval Response Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
        )
      }
    }
  }

  func setPluginEnabled(pluginID: String, enabled: Bool) {
    guard runtimeState == .ready else {
      return
    }

    Task {
      do {
        let updatedPlugin = try await runtimeBridge.setPluginEnabled(pluginID: pluginID, enabled: enabled)
        await refreshPluginState()
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: enabled ? "Plugin Enabled" : "Plugin Disabled",
            body: "\(updatedPlugin.displayName) is now \(enabled ? "enabled" : "disabled").",
            attributes: [
              "pluginId": updatedPlugin.id,
              "pluginStatus": updatedPlugin.status,
            ]
          )
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Plugin Update Failed",
            body: error.localizedDescription,
            attributes: [
              "pluginId": pluginID
            ]
          )
        )
      }
    }
  }

  func removePlugin(pluginID: String) {
    guard runtimeState == .ready,
          let plugin = plugins.first(where: { $0.id == pluginID }),
          plugin.provenance == "local"
    else {
      return
    }

    guard confirmPluginRemoval(plugin: plugin) else {
      runtimeDetail = "Plugin removal was cancelled."
      return
    }

    Task {
      do {
        let removedPlugin = try await runtimeBridge.removePlugin(manifestPath: plugin.manifestPath)
        await refreshPluginState()
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: "Plugin Removed",
            body:
              "\(removedPlugin.displayName) was removed from the local plugin catalog.\nRemoved Path: \(removedPlugin.removedPath)",
            attributes: [
              "pluginId": removedPlugin.pluginID,
              "removedPath": removedPlugin.removedPath,
            ]
          )
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Plugin Removal Failed",
            body: error.localizedDescription,
            attributes: [
              "pluginId": pluginID
            ]
          )
        )
      }
    }
  }

  func runPluginCommand(commandID: String) {
    guard runtimeState == .ready,
          isLocalModelReady(),
          let threadID = selectedThreadID,
          activeTurnID == nil
    else {
      return
    }

    Task {
      do {
        let result = try await runtimeBridge.runPluginCommand(threadID: threadID, commandID: commandID)
        appendItemsToTimeline(threadID: result.threadID, items: result.items)
        updatePendingApprovals(threadID: result.threadID, approvals: result.pendingApprovals)
        updateActiveTurn(threadID: result.threadID, activeTurnID: result.activeTurnID)
        refreshThreadPreview(
          threadID: result.threadID,
          preview: result.activeTurnID == nil ? "\(result.turnID) ready" : "Streaming response"
        )
        await refreshMemoryState()
      } catch {
        appendEntry(
          to: threadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Plugin Command Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
        )
      }
    }
  }

  func selectThread(id: String?) {
    selectedThreadID = id
    syncVisibleTimeline()

    guard runtimeState == .ready,
          let threadID = id,
          !threadID.hasPrefix("local-")
    else {
      return
    }

    Task {
      await loadThreadHistory(threadID: threadID)
      announceSetupCompleteIfNeeded()
    }
  }

  func saveWorkspaceMemoryNote() {
    let title = memoryNoteTitle.trimmingCharacters(in: .whitespacesAndNewlines)
    let body = memoryNoteBody.trimmingCharacters(in: .whitespacesAndNewlines)

    guard runtimeState == .ready,
          workspace != nil,
          !title.isEmpty,
          !body.isEmpty
    else {
      return
    }

    Task {
      do {
        let note = try await runtimeBridge.createMemoryNote(title: title, body: body)
        memoryNoteTitle = ""
        memoryNoteBody = ""
        await refreshMemoryState()
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: "Memory Note Saved",
            body: "Saved built-in workspace note \(note.title).",
            attributes: [
              "memoryNoteId": note.id,
              "memoryScope": note.scope,
              "memorySource": note.source,
            ]
          )
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Memory Note Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
        )
      }
    }
  }

  func cancelActiveTurn() {
    guard runtimeState == .ready,
          let activeTurnID,
          let activeTurnThreadID
    else {
      return
    }

    Task {
      do {
        let result = try await runtimeBridge.cancelTurn(turnID: activeTurnID)
        appendItemsToTimeline(threadID: result.threadID, items: result.items)
        updateActiveTurn(threadID: result.threadID, activeTurnID: result.activeTurnID)
        refreshThreadPreview(threadID: activeTurnThreadID, preview: "Cancelled response")
        await loadThreadHistory(threadID: result.threadID)
      } catch {
        appendEntry(
          to: activeTurnThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Turn Cancel Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
        )
      }
    }
  }

  func selectedThreadTitle() -> String {
    guard let selectedThreadID,
          let thread = threads.first(where: { $0.id == selectedThreadID })
    else {
      return "No Thread Selected"
    }

    return thread.title
  }

  func selectedThreadPreview() -> String {
    guard let selectedThreadID,
          let thread = threads.first(where: { $0.id == selectedThreadID })
    else {
      return "Select a thread to inspect its runtime state."
    }

    return thread.preview
  }

  func selectTimelineEntry(id: TimelineEntry.ID) {
    selectedEntryID = id
  }

  func selectedEntryTitle() -> String {
    selectedEntry()?.title ?? "No Item Selected"
  }

  func selectedEntryBody() -> String {
    selectedEntry()?.body ?? "Select a timeline item to inspect its details."
  }

  func selectedEntryMetadata() -> String {
    guard let entry = selectedEntry() else {
      return "No timeline item is selected."
    }

    if entry.attributes.isEmpty {
      return entry.kind.rawValue
    }

    let detail = entry.attributes
      .sorted(by: { $0.key < $1.key })
      .map { "\($0.key): \($0.value)" }
      .joined(separator: "\n")

    return "\(entry.kind.rawValue)\n\(detail)"
  }

  func selectedDiffSummary() -> String? {
    guard let entry = selectedEntry(), entry.kind == .diff else {
      return nil
    }

    let lines = diffLines(from: entry.body)
    let additions = lines.filter { $0.kind == .addition }.count
    let deletions = lines.filter { $0.kind == .deletion }.count
    let hunks = lines.filter { $0.kind == .hunk }.count
    let path = entry.attributes["relativePath"] ?? diffPathSummary(from: lines)
    return "\(path) | +\(additions) -\(deletions) | \(hunks) hunk\(hunks == 1 ? "" : "s")"
  }

  func selectedDiffLines() -> [DiffLineSummary] {
    guard let entry = selectedEntry(), entry.kind == .diff else {
      return []
    }

    return diffLines(from: entry.body)
  }

  func selectedEntryMemorySummary() -> String? {
    guard let entry = selectedEntry(),
          let noteCount = entry.attributes["memoryNoteCount"],
          noteCount != "0"
    else {
      return nil
    }

    let memoryTitles = entry.attributes["memoryNoteTitles"] ?? "Unavailable"
    let memoryIDs = entry.attributes["memoryNoteIds"] ?? "Unavailable"
    return "Notes: \(noteCount)\nTitles: \(memoryTitles)\nIDs: \(memoryIDs)"
  }

  func workspaceDisplayName() -> String {
    workspace?.displayName ?? "No Workspace"
  }

  func workspacePath() -> String {
    workspace?.rootPath ?? "Open a local workspace to enable Milestone 1 tools."
  }

  func modelDisplayName() -> String {
    modelHealth?.displayName ?? "Local Model Not Loaded"
  }

  func modelStatusSummary() -> String {
    guard let modelHealth else {
      return "Launch the runtime to inspect local model health."
    }

    return "\(modelHealth.backend) | \(modelHealth.status)"
  }

  func modelActionSummary() -> String {
    switch runtimeState {
    case .disconnected:
      return "Launch the runtime to inspect local model setup."
    case .launching:
      return "Checking local model setup..."
    case .failed:
      return "Relaunch the runtime before changing model setup."
    case .ready:
      if let modelDownloadID,
         let model = localModels.first(where: { $0.id == modelDownloadID })
      {
        return "Downloading \(model.displayName). You can pause or cancel without losing control."
      }

      if let pausedModelDownloadID,
         let model = localModels.first(where: { $0.id == pausedModelDownloadID })
      {
        return "\(model.displayName) is paused. Continue to resume or cancel to clear the partial file."
      }

      if isLocalModelReady() {
        return "Local model is ready for offline agent work."
      }

      let downloadedModels = localModels.filter { $0.downloaded }
      if downloadedModels.isEmpty {
        return "Download the default LFM2.5-350M model to unlock local agent work."
      }

      if downloadedModels.contains(where: { $0.id == "lfm2.5-350m" }) {
        return "Use the downloaded default model or reinstall pack metadata to repair readiness."
      }

      return "Select a downloaded model or download the default LFM2.5-350M baseline."
    }
  }

  func showsModelActivity() -> Bool {
    runtimeState == .launching || modelDownloadID != nil
  }

  func isModelActionBlocking() -> Bool {
    runtimeState == .failed
      || (runtimeState == .ready && !isLocalModelReady() && modelDownloadID == nil)
  }

  func modelDetailSummary() -> String {
    guard let modelHealth else {
      return "Pith will use the built-in local model path after the runtime connects."
    }

    return modelHealth.detail
  }

  func modelSourceSummary() -> String {
    guard let modelHealth else {
      return "Source: unavailable"
    }

    let source = "Source: \(modelHealth.source)"
    if let manifestPath = modelHealth.manifestPath {
      return "\(source)\nManifest: \(manifestPath)"
    }

    return source
  }

  func modelMetricsSummary() -> String {
    guard let modelHealth else {
      return "Metrics: unavailable"
    }

    let contextSize = modelHealth.metrics["contextSize"] ?? "unknown"
    let maxOutputTokens = modelHealth.metrics["maxOutputTokens"] ?? "unknown"
    let backend = modelHealth.metrics["backend"] ?? modelHealth.backend
    return "Context: \(contextSize) | Max Output: \(maxOutputTokens) | Backend: \(backend)"
  }

  func modelReadinessSummary() -> String {
    guard let modelHealth else {
      return "Readiness: unavailable"
    }

    let readiness = modelHealth.metrics["readiness"] ?? "unknown"
    let packReady = modelHealth.metrics["packReady"] ?? "false"
    return "Readiness: \(readiness) | Pack Ready: \(packReady)"
  }

  func modelInstallHintSummary() -> String {
    guard let modelHealth else {
      return "Install hint: launch the runtime to inspect local model setup."
    }

    return modelHealth.metrics["installHint"] ?? "Install hint unavailable."
  }

  func modelSuggestedPathSummary() -> String {
    guard let modelHealth else {
      return "Suggested install layout unavailable."
    }

    let manifestPath = modelHealth.metrics["suggestedManifestPath"] ?? "manifest path unavailable"
    let modelPath = modelHealth.metrics["suggestedModelPath"] ?? "model path unavailable"
    let binaryPath = modelHealth.metrics["suggestedBinaryPath"] ?? "binary path unavailable"
    return "Suggested Manifest: \(manifestPath)\nSuggested Model: \(modelPath)\nSuggested Binary: \(binaryPath)"
  }

  func modelArtifactPathSummary() -> String {
    guard let modelHealth else {
      return "No local model paths available yet."
    }

    let modelPath = modelHealth.modelPath ?? "model path unavailable"
    let binaryPath = modelHealth.binaryPath ?? "binary path unavailable"
    let manifestPath = modelHealth.manifestPath ?? "manifest path unavailable"
    return "Model: \(modelPath)\nBinary: \(binaryPath)\nManifest: \(manifestPath)"
  }

  func modelManagerSummary() -> String {
    let downloadedModels = localModels.filter { $0.downloaded }
    let activeModel = localModels.first(where: { $0.active })?.displayName ?? "none"
    let downloadingModel = modelDownloadID
      .flatMap { id in localModels.first(where: { $0.id == id })?.displayName }
    let pausedModel = pausedModelDownloadID
      .flatMap { id in localModels.first(where: { $0.id == id })?.displayName }
    let localSize = downloadedModels
      .compactMap { $0.localSizeBytes }
      .reduce(Int64(0), +)
    let downloadSummary = downloadingModel.map { " | Downloading: \($0)" } ?? ""
    let pausedSummary = pausedModel.map { " | Paused: \($0)" } ?? ""
    return "Downloaded: \(downloadedModels.count)/\(localModels.count) | Local Size: \(formattedByteCount(localSize)) | Active: \(activeModel)\(downloadSummary)\(pausedSummary)"
  }

  func shouldShowModelDownloadProgress() -> Bool {
    guard let modelDownloadProgress else {
      return false
    }

    return modelDownloadID == modelDownloadProgress.modelID
      || pausedModelDownloadID == modelDownloadProgress.modelID
  }

  func modelDownloadProgressValue() -> Double? {
    guard let modelDownloadProgress,
          modelDownloadProgress.totalBytes > 0
    else {
      return nil
    }

    let value = Double(modelDownloadProgress.bytesReceived)
      / Double(modelDownloadProgress.totalBytes)
    return min(max(value, 0), 1)
  }

  func modelDownloadProgressSummary() -> String {
    guard let modelDownloadProgress else {
      return ""
    }

    let received = formattedByteCount(modelDownloadProgress.bytesReceived)
    let total = modelDownloadProgress.totalBytes > 0
      ? formattedByteCount(modelDownloadProgress.totalBytes)
      : "unknown size"
    let isPaused = pausedModelDownloadID == modelDownloadProgress.modelID
    let status = isPaused ? "Paused" : (modelDownloadProgress.isResuming ? "Continuing" : "Downloading")
    let trailingStatus = isPaused ? "Ready to continue" : modelDownloadSpeedSummary(modelDownloadProgress)
    let percent = modelDownloadProgressValue()
      .map { " | \(Int($0 * 100))%" }
      ?? ""

    return "\(status) \(modelDownloadProgress.displayName): \(received) of \(total)\(percent) | \(trailingStatus)"
  }

  func localModelStatusSummary(_ model: LocalModelSummary) -> String {
    let status: String
    if modelDownloadID == model.id {
      status = "downloading"
    } else if pausedModelDownloadID == model.id {
      status = "paused"
    } else if model.active {
      status = "active"
    } else if model.downloaded {
      status = "downloaded"
    } else {
      status = "available"
    }

    let localSize = model.localSizeBytes.map(formattedByteCount) ?? formattedByteCount(model.sizeBytes)
    return "\(status) | \(localSize) | \(model.license)"
  }

  func defaultModelDownloadButtonTitle() -> String {
    if modelDownloadID == "lfm2.5-350m" {
      return "Downloading Model"
    }
    if pausedModelDownloadID == "lfm2.5-350m" {
      return "Continue Model"
    }
    if let defaultModel = localModels.first(where: { $0.id == "lfm2.5-350m" }) {
      if defaultModel.active {
        return "Model Selected"
      }
      if defaultModel.downloaded {
        return "Use Downloaded Model"
      }
    }

    return "Download Model"
  }

  func localModelDownloadButtonTitle(_ model: LocalModelSummary) -> String {
    if modelDownloadID == model.id {
      return "Downloading"
    }
    if pausedModelDownloadID == model.id {
      return "Continue"
    }

    return model.downloaded ? "Downloaded" : "Download"
  }

  func localModelTagSummary(_ model: LocalModelSummary) -> String {
    model.tags.joined(separator: " / ")
  }

  func localModelPathSummary(_ model: LocalModelSummary) -> String {
    model.installPath
  }

  func canDownloadRecommendedModel(modelID: String) -> Bool {
    guard modelDownloadTask == nil else {
      return false
    }
    if let pausedModelDownloadID {
      return pausedModelDownloadID == modelID && modelDownloadResumeData != nil
    }
    guard let model = localModels.first(where: { $0.id == modelID }),
          URL(string: model.downloadURL) != nil
    else {
      return false
    }

    return !model.downloaded
  }

  func canActivateRecommendedModel(modelID: String) -> Bool {
    guard runtimeState != .launching,
          modelDownloadTask == nil,
          pausedModelDownloadID == nil
    else {
      return false
    }
    guard let model = localModels.first(where: { $0.id == modelID }) else {
      return false
    }

    return model.downloaded && !model.active
  }

  func canResetActiveLocalModel() -> Bool {
    runtimeState != .launching
      && modelDownloadTask == nil
      && runtimeBridge.activeLocalModelPath() != nil
  }

  func canCancelModelDownload() -> Bool {
    modelDownloadTask != nil || pausedModelDownloadID != nil
  }

  func canPauseModelDownload() -> Bool {
    modelDownloadTask != nil
  }

  func pauseModelDownload() {
    let displayName = modelDownloadID
      .flatMap { id in localModels.first(where: { $0.id == id })?.displayName }
      ?? "local model"
    runtimeDetail = "Pausing \(displayName) download..."
    modelDownloadTransfer?.pause()
  }

  func cancelModelDownload() {
    if let modelDownloadTask {
      let displayName = modelDownloadID
        .flatMap { id in localModels.first(where: { $0.id == id })?.displayName }
        ?? "local model"
      runtimeDetail = "Cancelling \(displayName) download..."
      modelDownloadTask.cancel()
      modelDownloadTransfer?.cancel()
      return
    }

    guard let pausedModelDownloadID else {
      return
    }

    let displayName = localModels.first(where: { $0.id == pausedModelDownloadID })?.displayName
      ?? "local model"
    clearPausedModelDownload()
    removeIncompleteModelFile(modelID: pausedModelDownloadID)
    modelDownloadProgress = nil
    runtimeDetail = "Cancelled \(displayName) download and cleared partial state."
    refreshLocalModelCatalog()
    if let model = localModels.first(where: { $0.id == pausedModelDownloadID }) {
      appendModelEvent(
        title: "Local Model Download Cancelled",
        body: "\(model.displayName) download was cancelled and the partial file was cleared.",
        model: model,
        attributes: [
          "result": "cancelled"
        ]
      )
    }
  }

  func downloadRecommendedModel(modelID: String) {
    guard let model = localModels.first(where: { $0.id == modelID }) else {
      runtimeDetail = "The selected local model is unavailable."
      return
    }

    guard let downloadURL = URL(string: model.downloadURL) else {
      runtimeDetail = "The selected local model has an invalid download URL."
      return
    }

    let isResuming = pausedModelDownloadID == model.id && modelDownloadResumeData != nil
    if !isResuming {
      guard confirmModelDownload(
        displayName: model.displayName,
        downloadURL: downloadURL,
        targetPath: model.installPath,
        sizeSummary: formattedByteCount(model.sizeBytes)
      ) else {
        runtimeDetail = "Local model download was cancelled."
        return
      }
    }

    let resumeData = isResuming ? modelDownloadResumeData : nil
    let resumedBytes = isResuming && modelDownloadProgress?.modelID == model.id
      ? modelDownloadProgress?.bytesReceived ?? 0
      : 0
    modelDownloadID = model.id
    pausedModelDownloadID = nil
    modelDownloadResumeData = nil
    modelDownloadProgress = ModelDownloadProgress(
      modelID: model.id,
      displayName: model.displayName,
      bytesReceived: resumedBytes,
      totalBytes: model.sizeBytes,
      startedAt: Date(),
      updatedAt: Date(),
      isResuming: isResuming
    )
    appendModelEvent(
      title: isResuming ? "Local Model Download Continued" : "Local Model Download Started",
      body:
        "\(model.displayName) download \(isResuming ? "continued" : "started") from \(downloadURL.absoluteString).",
      model: model,
      attributes: [
        "downloadUrl": downloadURL.absoluteString,
        "size": formattedByteCount(model.sizeBytes)
      ]
    )
    modelDownloadTask = Task {
      defer {
        modelDownloadID = nil
        modelDownloadTask = nil
        modelDownloadTransfer = nil
        refreshLocalModelCatalog()
      }
      do {
        runtimeDetail =
          "\(isResuming ? "Continuing" : "Downloading") \(model.displayName) (\(formattedByteCount(model.sizeBytes)))..."
        try await downloadModelFile(
          from: downloadURL,
          resumeData: resumeData,
          modelID: model.id,
          expectedBytes: model.sizeBytes,
          to: URL(fileURLWithPath: model.installPath)
        )

        var activatedDefaultModel = false
        var manifestPath: String?
        if model.id == "lfm2.5-350m" {
          let defaultManifestPath = try writeLocalModelPackManifest(for: model)
          runtimeBridge.configureActiveLocalModel(
            manifestPath: defaultManifestPath,
            modelPath: model.installPath
          )
          manifestPath = defaultManifestPath
          activatedDefaultModel = true
        }

        if activatedDefaultModel {
          runtimeDetail = "Downloaded and selected \(model.displayName)."
          modelDownloadProgress = nil
          refreshLocalModelCatalog()
        } else {
          runtimeDetail = "Downloaded \(model.displayName) to \(model.installPath)."
          modelDownloadProgress = nil
          refreshLocalModelCatalog()
        }

        var attributes = [
          "modelPath": model.installPath,
          "source": downloadURL.absoluteString,
        ]
        if let manifestPath {
          attributes["manifestPath"] = manifestPath
        }

        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: "Local Model Downloaded",
            body: activatedDefaultModel
              ? "\(model.displayName) was downloaded and selected as the active local model."
              : "\(model.displayName) was downloaded to \(model.installPath).",
            attributes: attributes
          )
        )

        if activatedDefaultModel {
          relaunchRuntimeIfNeeded(
            runningDetail: "Restarting local runtime with \(model.displayName)...",
            idleDetail: "\(model.displayName) will be used when the runtime launches."
          )
        }
      } catch {
        if let paused = error as? ModelDownloadPaused {
          modelDownloadResumeData = paused.resumeData
          pausedModelDownloadID = model.id
          runtimeDetail = "Paused \(model.displayName) download. Continue to resume from the saved partial state."
          appendModelEvent(
            title: "Local Model Download Paused",
            body: "\(model.displayName) download was paused and can continue from the saved partial state.",
            model: model,
            attributes: [
              "result": "paused"
            ]
          )
        } else if error is CancellationError || (error as? URLError)?.code == .cancelled {
          clearPausedModelDownload()
          removeIncompleteModelFile(modelID: model.id)
          modelDownloadProgress = nil
          runtimeDetail = "Cancelled \(model.displayName) download and cleared partial state."
          appendModelEvent(
            title: "Local Model Download Cancelled",
            body: "\(model.displayName) download was cancelled and the partial file was cleared.",
            model: model,
            attributes: [
              "result": "cancelled"
            ]
          )
        } else {
          clearPausedModelDownload()
          modelDownloadProgress = nil
          runtimeDetail = "Model download failed: \(error.localizedDescription)"
          appendModelEvent(
            title: "Local Model Download Failed",
            body: "\(model.displayName) download failed: \(error.localizedDescription)",
            model: model,
            kind: .warning,
            attributes: [
              "result": "failed"
            ]
          )
        }
      }
    }
  }

  func activateRecommendedModel(modelID: String) {
    guard let model = localModels.first(where: { $0.id == modelID }) else {
      runtimeDetail = "The selected local model is unavailable."
      return
    }

    guard model.downloaded else {
      runtimeDetail = "Download \(model.displayName) before using it."
      return
    }

    do {
      let manifestPath = try writeLocalModelPackManifest(for: model)
      runtimeBridge.configureActiveLocalModel(
        manifestPath: manifestPath,
        modelPath: model.installPath
      )
      refreshLocalModelCatalog()
      appendEntry(
        to: selectedThreadID,
        TimelineEntry(
          id: UUID().uuidString,
          kind: .system,
          title: "Local Model Selected",
          body: "\(model.displayName) is now the active local model.",
          attributes: [
            "modelId": model.id,
            "manifestPath": manifestPath,
            "modelPath": model.installPath,
          ]
        )
      )
      relaunchRuntimeIfNeeded(
        runningDetail: "Restarting local runtime with \(model.displayName)...",
        idleDetail: "\(model.displayName) will be used when the runtime launches."
      )
    } catch {
      runtimeDetail = "Model selection failed: \(error.localizedDescription)"
    }
  }

  func resetActiveLocalModel() {
    runtimeBridge.clearActiveLocalModel()
    refreshLocalModelCatalog()
    appendEntry(
      to: selectedThreadID,
      TimelineEntry(
        id: UUID().uuidString,
        kind: .system,
        title: "Local Model Reset",
        body: "Pith will use the default local model discovery path.",
        attributes: [:]
      )
    )
    relaunchRuntimeIfNeeded(
      runningDetail: "Restarting local runtime with default model discovery...",
      idleDetail: "Default model discovery will be used when the runtime launches."
    )
  }

  func revealRecommendedModel(modelID: String) {
    guard let model = localModels.first(where: { $0.id == modelID }) else {
      runtimeDetail = "The selected local model is unavailable."
      return
    }

    revealFilePath(model.installPath, successDetail: "Revealed \(model.displayName).")
  }

  func revealSuggestedModelDirectory() {
    revealSuggestedPath(
      metricKey: "suggestedModelPath",
      successDetail: "Opened the suggested local model folder."
    )
  }

  func revealSuggestedBinaryDirectory() {
    revealSuggestedPath(
      metricKey: "suggestedBinaryPath",
      successDetail: "Opened the suggested llama.cpp binary folder."
    )
  }

  func canDownloadLocalModel() -> Bool {
    canDownloadRecommendedModel(modelID: "lfm2.5-350m")
      || canActivateRecommendedModel(modelID: "lfm2.5-350m")
  }

  func downloadLocalModel() {
    if canActivateRecommendedModel(modelID: "lfm2.5-350m") {
      activateRecommendedModel(modelID: "lfm2.5-350m")
      return
    }

    downloadRecommendedModel(modelID: "lfm2.5-350m")
  }

  func canBootstrapModelPackMetadata() -> Bool {
    runtimeState == .ready && modelDownloadTask == nil
  }

  func bootstrapModelPackMetadata() {
    guard runtimeState == .ready else {
      runtimeDetail = "Launch the runtime before preparing local model metadata."
      return
    }

    Task {
      do {
        let result = try await runtimeBridge.bootstrapModelPack()
        await refreshModelHealthState()
        let copiedSummary = result.copiedFiles.isEmpty
          ? "Pack metadata was already present."
          : "Prepared \(result.copiedFiles.count) local model metadata file(s)."
        runtimeDetail = "\(copiedSummary) Manifest: \(result.manifestPath)"
      } catch {
        runtimeDetail = "Model metadata bootstrap failed: \(error.localizedDescription)"
      }
    }
  }

  func pluginCountSummary() -> String {
    if plugins.isEmpty {
      return "No bundled plugins discovered yet."
    }

    let readyCount = plugins.filter { $0.status == "ready" }.count
    let invalidCount = plugins.count - readyCount
    if invalidCount == 0 {
      return "\(readyCount) plugin(s) discovered"
    }

    return "\(readyCount) ready, \(invalidCount) invalid"
  }

  func localPluginCountSummary() -> String {
    let localPlugins = plugins.filter { $0.provenance == "local" }

    if localPlugins.isEmpty {
      return "No local plugin installs yet."
    }

    return "\(localPlugins.count) local plugin install\(localPlugins.count == 1 ? "" : "s")"
  }

  func pluginDetailSummary() -> String {
    guard !plugins.isEmpty else {
      return "Pith discovers plugin manifests from the bundled plugins directory."
    }

    return plugins
      .map { plugin in
        let capabilities = plugin.capabilities.isEmpty ? "none" : plugin.capabilities.joined(separator: ", ")
        let validation = plugin.validationError ?? "ok"
        let hint = plugin.validationHint.map { " | repair: \($0)" } ?? ""
        return "\(plugin.displayName) \(plugin.version) | \(plugin.status) | \(plugin.provenance) | capabilities: \(capabilities) | validation: \(validation)\(hint)"
      }
      .joined(separator: "\n")
  }

  func pluginPermissionCountSummary() -> String {
    let readyPlugins = plugins.filter { $0.status == "ready" }
    let uniquePermissions = Set(readyPlugins.flatMap(\.permissions))

    guard !readyPlugins.isEmpty else {
      return "Plugin permissions are not loaded yet."
    }

    if uniquePermissions.isEmpty {
      return "\(readyPlugins.count) ready plugin(s), no declared permissions"
    }

    return "\(uniquePermissions.count) permission(s) across \(readyPlugins.count) ready plugin(s)"
  }

  func pluginPermissionDetailSummary() -> String {
    let readyPlugins = plugins.filter { $0.status == "ready" }

    guard !readyPlugins.isEmpty else {
      return "Permission coverage appears here after the runtime loads plugin manifests."
    }

    let uniquePermissions = Set(readyPlugins.flatMap(\.permissions))
    if uniquePermissions.isEmpty {
      return "The current ready plugins do not declare extra runtime permissions."
    }

    return uniquePermissions
      .sorted()
      .map { permission in
        let grantingPlugins = readyPlugins
          .filter { $0.permissions.contains(permission) }
          .map(\.displayName)
          .sorted()
          .joined(separator: ", ")
        return "\(permission): \(grantingPlugins)"
      }
      .joined(separator: "\n")
  }

  func pluginPermissionPreview() -> [PluginSummary] {
    plugins.filter { $0.status == "ready" }
  }

  func invalidPluginCountSummary() -> String {
    let invalidPlugins = plugins.filter { $0.status != "ready" }

    if invalidPlugins.isEmpty {
      return "No Manifest Issues"
    }

    return "\(invalidPlugins.count) Invalid Plugin Manifest\(invalidPlugins.count == 1 ? "" : "s")"
  }

  func invalidPluginDetailSummary() -> String {
    let invalidPlugins = plugins.filter { $0.status != "ready" }

    guard !invalidPlugins.isEmpty else {
      return "All discovered plugin manifests match the current runtime schema."
    }

    return invalidPlugins
      .map { plugin in
        let hint = plugin.validationHint.map { " Repair hint: \($0)" } ?? ""
        return "\(plugin.displayName): \(plugin.validationError ?? "Unknown validation error")\(hint)"
      }
      .joined(separator: "\n")
  }

  func invalidPlugins() -> [PluginSummary] {
    plugins.filter { $0.status != "ready" }
  }

  func isRemovablePlugin(_ plugin: PluginSummary) -> Bool {
    plugin.provenance == "local"
  }

  func revealPluginManifest(pluginID: String) {
    guard let plugin = plugins.first(where: { $0.id == pluginID }) else {
      runtimeDetail = "Plugin manifest path is unavailable."
      return
    }

    revealFilePath(plugin.manifestPath, successDetail: "Revealed \(plugin.displayName) manifest.")
  }

  func pluginRegistryCountSummary() -> String {
    guard let pluginCapabilityRegistrySummary else {
      return "Capability registry not loaded yet."
    }

    return
      "\(pluginCapabilityRegistrySummary.totalCapabilityCount) capability(ies) from \(pluginCapabilityRegistrySummary.enabledPluginCount) enabled plugin(s)"
  }

  func pluginRegistryDetailSummary() -> String {
    guard let pluginCapabilityRegistrySummary else {
      return "Enable a ready plugin to populate the typed capability registry."
    }

    let kindSummary = pluginCapabilityRegistrySummary.capabilityCountsByKind
      .sorted(by: { $0.key < $1.key })
      .map { "\($0.key): \($0.value)" }
      .joined(separator: " | ")
    if kindSummary.isEmpty {
      return "No capabilities are currently registered."
    }

    return kindSummary
  }

  func pluginCapabilityPreview() -> [PluginCapabilitySummary] {
    Array(pluginCapabilities.prefix(6))
  }

  func pluginConnectorCountSummary() -> String {
    if pluginConnectors.isEmpty {
      return "No Connectors"
    }

    return "\(pluginConnectors.count) Connector\(pluginConnectors.count == 1 ? "" : "s")"
  }

  func pluginConnectorDetailSummary() -> String {
    guard !pluginConnectors.isEmpty else {
      return "Install or enable connector plugins to prepare third-party app integrations."
    }

    return pluginConnectors
      .map { "\($0.displayName): \($0.status) via \($0.pluginDisplayName)" }
      .joined(separator: "\n")
  }

  func pluginConnectorPreview() -> [PluginConnectorSummary] {
    Array(pluginConnectors.prefix(6))
  }

  func pluginCommandCountSummary() -> String {
    if pluginCommands.isEmpty {
      return "No Plugin Commands"
    }

    return "\(pluginCommands.count) Plugin Command\(pluginCommands.count == 1 ? "" : "s")"
  }

  func pluginCommandDetailSummary() -> String {
    guard !pluginCommands.isEmpty else {
      return "Enable ready plugins with declared command capabilities to run reusable local workflows."
    }

    return pluginCommands
      .map { "\($0.pluginDisplayName): \($0.title)" }
      .joined(separator: "\n")
  }

  func pluginHookCountSummary() -> String {
    if pluginHooks.isEmpty {
      return "No Plugin Hooks"
    }

    return "\(pluginHooks.count) Plugin Hook\(pluginHooks.count == 1 ? "" : "s")"
  }

  func pluginHookDetailSummary() -> String {
    guard !pluginHooks.isEmpty else {
      return "Enable ready plugins with declared hook capabilities to extend local runtime events."
    }

    return pluginHooks
      .map { "\($0.pluginDisplayName): \($0.title) (\($0.event))" }
      .joined(separator: "\n")
  }

  func memoryCountSummary() -> String {
    guard let memoryStatus else {
      return "Built-in memory is not connected yet."
    }

    return "\(memoryStatus.noteCount) note(s) captured"
  }

  func memoryDetailSummary() -> String {
    guard let memoryStatus else {
      return "Pith uses built-in memory instead of a memory plugin. Workspace notes are stored locally by the runtime."
    }

    if memoryNotes.isEmpty {
      return memoryStatus.summary
    }

    return memoryNotes
      .prefix(4)
      .map { note in
        let tagSummary = note.tags.isEmpty ? "untagged" : note.tags.joined(separator: ", ")
        return "\(note.title) | \(note.scope) | \(note.source) | tags: \(tagSummary)"
      }
      .joined(separator: "\n")
  }

  func memoryLatestSummary() -> String {
    guard let latestNote = memoryNotes.first else {
      return "No memory notes have been captured yet."
    }

    return "\(latestNote.body)\nSource: \(latestNote.source)"
  }

  func canSaveWorkspaceMemoryNote() -> Bool {
    runtimeState == .ready
      && workspace != nil
      && !memoryNoteTitle.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
      && !memoryNoteBody.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
  }

  func isLocalModelReady() -> Bool {
    guard runtimeState == .ready,
          let modelHealth,
          modelHealth.status == "ready"
    else {
      return false
    }

    return (modelHealth.metrics["readiness"] ?? "unknown") == "configured"
  }

  func composerPlaceholder() -> String {
    switch runtimeState {
    case .disconnected:
      return "Launch the local runtime to start"
    case .launching:
      return "Runtime is starting..."
    case .failed:
      return "Relaunch the runtime to recover"
    case .ready:
      break
    }

    if !isLocalModelReady() {
      if modelDownloadID != nil {
        return "Model download is running..."
      }
      if pausedModelDownloadID != nil {
        return "Continue the paused model download"
      }
      if isDefaultModelDownloaded() {
        return "Use the downloaded local model"
      }
      return "Download the local LFM2.5-350M model"
    }

    if workspace == nil {
      return "Open a workspace to start local agent work"
    }

    if !hasRuntimeThreadSelection() {
      return "Create or select a thread"
    }

    if activeTurnID != nil {
      return "Pith is streaming a response. Cancel to stop the current turn."
    }

    return "Ask Pith to inspect files, review diffs, run shell commands, or write files"
  }

  func composerStatusSummary() -> String {
    switch runtimeState {
    case .disconnected:
      return "Launch the local runtime before starting agent work."
    case .launching:
      return "Launching the local runtime..."
    case .failed:
      return "Runtime is unavailable. Relaunch it to recover the local agent loop."
    case .ready:
      if !isLocalModelReady() {
        if modelDownloadID != nil {
          return "Model download is running. Agent work unlocks after the local model is ready."
        }
        if pausedModelDownloadID != nil {
          return "Model download is paused. Continue it from Local Model."
        }
        if isDefaultModelDownloaded() {
          return "Use the downloaded local model to finish setup."
        }
        return "Download the local LFM2.5-350M model to enable offline agent work."
      }

      if workspace == nil {
        return "Open a workspace to bind threads and tools to a local project."
      }

      if !hasRuntimeThreadSelection() {
        return "Create or select a thread to start local agent work."
      }

      if activeTurnID != nil {
        return "Pith is streaming locally. Cancel the turn if it is no longer useful."
      }

      return "Ready for local agent work. Press Command-Return to send."
    }
  }

  func showsComposerActivity() -> Bool {
    runtimeState == .launching || activeTurnID != nil
  }

  func isTurnStreaming() -> Bool {
    activeTurnID != nil
  }

  func isPendingApproval(_ entry: TimelineEntry) -> Bool {
    guard entry.kind == .approval,
          let selectedThreadID,
          let approvalID = entry.attributes["approvalId"]
    else {
      return false
    }

    return threadPendingApprovalIDs[selectedThreadID, default: Set<String>()].contains(approvalID)
  }

  func approvalID(for entry: TimelineEntry) -> String? {
    entry.attributes["approvalId"]
  }

  private func appendItemsToTimeline(
    threadID: String,
    items: [RuntimeBridge.RuntimeTimelineItemResult]
  ) {
    let newEntries = items.map { item in
      TimelineEntry(
        id: UUID().uuidString,
        kind: timelineKind(for: item.kind),
        title: item.title,
        body: item.content,
        attributes: item.attributes
      )
    }

    for entry in newEntries.reversed() {
      appendEntry(to: threadID, entry)
    }
  }

  private func updatePendingApprovals(
    threadID: String,
    approvals: [RuntimeBridge.RuntimeApproval]
  ) {
    threadPendingApprovalIDs[threadID] = Set(approvals.map(\.id))
  }

  private func refreshThreadPreview(threadID: String, preview: String) {
    guard let index = threads.firstIndex(where: { $0.id == threadID }) else {
      return
    }

    threads[index].preview = preview
  }

  private func appendEntry(to threadID: String?, _ entry: TimelineEntry) {
    guard let threadID else {
      timeline.insert(entry, at: 0)
      if selectedEntryID == nil {
        selectedEntryID = entry.id
      }
      return
    }

    var entries = threadTimelines[threadID] ?? defaultTimeline(for: threadTitle(for: threadID))
    entries.insert(entry, at: 0)
    threadTimelines[threadID] = entries

    if selectedThreadID == threadID {
      timeline = entries
      if !entries.contains(where: { $0.id == selectedEntryID }) {
        selectedEntryID = entries.first?.id
      }
    }
  }

  private func syncVisibleTimeline() {
    guard let selectedThreadID else {
      timeline = []
      selectedEntryID = nil
      return
    }

    timeline =
      threadTimelines[selectedThreadID]
      ?? defaultTimeline(for: threadTitle(for: selectedThreadID))
    threadTimelines[selectedThreadID] = timeline
    selectedEntryID = timeline.first?.id
  }

  private func defaultTimeline(for title: String) -> [TimelineEntry] {
    [
      TimelineEntry(
        id: UUID().uuidString,
        kind: .system,
        title: "Thread Ready",
        body: "\(title) is ready after runtime, model, workspace, and thread setup are complete.",
        attributes: [
          "setup": "runtime, model, workspace, thread"
        ]
      ),
    ]
  }

  private func threadTitle(for threadID: String) -> String {
    threads.first(where: { $0.id == threadID })?.title ?? "Thread"
  }

  private func loadThreadHistory(threadID: String) async {
    do {
      let result = try await runtimeBridge.readThread(threadID: threadID)
      let previousSelectionID = selectedThreadID == threadID ? selectedEntryID : nil
      let entries = timelineEntries(from: result.items)
      threadTimelines[threadID] = entries
      updatePendingApprovals(threadID: threadID, approvals: result.pendingApprovals)
      updateActiveTurn(threadID: threadID, activeTurnID: result.activeTurnID)
      refreshThreadPreview(threadID: threadID, preview: result.status)

      if selectedThreadID == threadID {
        timeline = entries
        if let previousSelectionID,
           entries.contains(where: { $0.id == previousSelectionID }) {
          selectedEntryID = previousSelectionID
        } else {
          selectedEntryID = entries.first?.id
        }
      }
    } catch {
      appendEntry(
        to: threadID,
        TimelineEntry(
          id: UUID().uuidString,
          kind: .warning,
          title: "Thread Load Failed",
          body: error.localizedDescription,
          attributes: [:]
        )
      )
    }
  }

  private func timelineKind(for rawKind: String) -> TimelineEntry.Kind {
    switch rawKind {
    case "userMessage":
      return .userMessage
    case "assistantMessage":
      return .assistantMessage
    case "plan":
      return .plan
    case "diffArtifact":
      return .diff
    case "toolStart", "toolResult", "pluginCommand", "pluginResult":
      return .tool
    case "approvalRequested", "approvalResolved":
      return .approval
    case "warning":
      return .warning
    default:
      return .system
    }
  }

  private func selectedEntry() -> TimelineEntry? {
    guard let selectedEntryID else {
      return nil
    }

    return timeline.first(where: { $0.id == selectedEntryID })
  }

  private func diffLines(from body: String) -> [DiffLineSummary] {
    body
      .components(separatedBy: .newlines)
      .enumerated()
      .map { index, line in
        DiffLineSummary(
          id: "\(index)",
          lineNumber: index + 1,
          text: line,
          kind: diffLineKind(for: line)
        )
      }
  }

  private func diffLineKind(for line: String) -> DiffLineKind {
    if line.hasPrefix("@@") {
      return .hunk
    }

    if line.hasPrefix("diff --git")
      || line.hasPrefix("index ")
      || line.hasPrefix("+++")
      || line.hasPrefix("---")
    {
      return .metadata
    }

    if line.hasPrefix("+") {
      return .addition
    }

    if line.hasPrefix("-") {
      return .deletion
    }

    return .context
  }

  private func diffPathSummary(from lines: [DiffLineSummary]) -> String {
    let pathLine = lines.first { line in
      line.text.hasPrefix("+++ b/") || line.text.hasPrefix("--- a/")
    }

    guard let pathLine else {
      return "Diff"
    }

    return pathLine.text
      .replacingOccurrences(of: "+++ b/", with: "")
      .replacingOccurrences(of: "--- a/", with: "")
  }

  private func updateActiveTurn(threadID: String, activeTurnID: String?) {
    if activeTurnID == nil {
      if activeTurnThreadID == threadID {
        activeTurnThreadID = nil
      }
      self.activeTurnID = nil
      return
    }

    if self.activeTurnID == activeTurnID, activeTurnThreadID == threadID {
      return
    }

    self.activeTurnID = activeTurnID
    activeTurnThreadID = threadID
  }

  private func refreshModelHealthState(serverLabel: String? = nil) async {
    let runtimeModel = try? await runtimeBridge.modelHealth()
    if let runtimeModel {
      modelHealth = ModelHealthSummary(
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
      if let serverLabel {
        runtimeDetail = "\(serverLabel) | \(runtimeModel.displayName)"
      }
      refreshLocalModelCatalog()
    } else {
      modelHealth = nil
      refreshLocalModelCatalog()
      if let serverLabel {
        runtimeDetail = serverLabel
      }
    }
    announceSetupCompleteIfNeeded()
  }

  private func refreshLocalModelCatalog() {
    let activeModelPath = runtimeBridge.activeLocalModelPath() ?? modelHealth?.modelPath
    localModels = Self.localModelSummaries(
      storageRootPath: runtimeBridge.localModelStorageRootPath(),
      activeModelPath: activeModelPath
    )
  }

  private func setupReadyStepCount() -> Int {
    var readyCount = 0
    if runtimeState == .ready {
      readyCount += 1
    }
    if isLocalModelReady() {
      readyCount += 1
    }
    if workspace != nil {
      readyCount += 1
    }
    if hasRuntimeThreadSelection() {
      readyCount += 1
    }
    return readyCount
  }

  private func hasRuntimeThreadSelection() -> Bool {
    guard let selectedThreadID else {
      return false
    }

    return !selectedThreadID.hasPrefix("local-")
  }

  private func isDefaultModelDownloaded() -> Bool {
    defaultLocalModel()?.downloaded == true
  }

  private func defaultLocalModel() -> LocalModelSummary? {
    localModels.first(where: { $0.id == "lfm2.5-350m" })
  }

  private func localModelRequiredTimelineSummary() -> String {
    if modelDownloadID != nil {
      return "The local model is downloading. Pith will unlock agent work after the model is ready."
    }
    if pausedModelDownloadID != nil {
      return "The local model download is paused. Continue the download to finish first-use setup."
    }
    if isDefaultModelDownloaded() {
      return "The default model is downloaded but not active. Use the downloaded model to finish first-use setup."
    }

    return "No ready local model is installed yet. Download the LFM2.5-350M model to finish first-use setup without an external API."
  }

  private func announceSetupCompleteIfNeeded() {
    guard setupReadyStepCount() == setupStepCount,
          let threadID = selectedThreadID,
          !threadID.hasPrefix("local-"),
          !announcedSetupCompleteThreadIDs.contains(threadID)
    else {
      return
    }

    announcedSetupCompleteThreadIDs.insert(threadID)
    appendEntry(
      to: threadID,
      TimelineEntry(
        id: UUID().uuidString,
        kind: .system,
        title: "Local Setup Complete",
        body: "Runtime, local model, workspace, and thread are ready. Ask Pith to inspect files, review diffs, or make a small change.",
        attributes: [
          "setup": "complete"
        ]
      )
    )
  }

  private func runtimeReadinessStep() -> ReadinessStepSummary {
    switch runtimeState {
    case .ready:
      return ReadinessStepSummary(id: "runtime", label: "Runtime", detail: "Ready", tone: .ready)
    case .launching:
      return ReadinessStepSummary(id: "runtime", label: "Runtime", detail: "Starting", tone: .active)
    case .failed:
      return ReadinessStepSummary(id: "runtime", label: "Runtime", detail: "Relaunch", tone: .danger)
    case .disconnected:
      return ReadinessStepSummary(id: "runtime", label: "Runtime", detail: "Launch", tone: .warning)
    }
  }

  private func workspaceReadinessStep() -> ReadinessStepSummary {
    guard runtimeState == .ready else {
      return ReadinessStepSummary(id: "workspace", label: "Workspace", detail: "Waiting", tone: .neutral)
    }
    guard let workspace else {
      return ReadinessStepSummary(id: "workspace", label: "Workspace", detail: "Open", tone: .warning)
    }

    return ReadinessStepSummary(
      id: "workspace",
      label: "Workspace",
      detail: workspace.displayName,
      tone: .ready
    )
  }

  private func modelReadinessStep() -> ReadinessStepSummary {
    guard runtimeState == .ready else {
      return ReadinessStepSummary(id: "model", label: "Model", detail: "Waiting", tone: .neutral)
    }
    if modelDownloadID != nil {
      return ReadinessStepSummary(id: "model", label: "Model", detail: "Downloading", tone: .active)
    }
    if pausedModelDownloadID != nil {
      return ReadinessStepSummary(id: "model", label: "Model", detail: "Paused", tone: .warning)
    }
    if isLocalModelReady() {
      return ReadinessStepSummary(id: "model", label: "Model", detail: "Ready", tone: .ready)
    }
    if isDefaultModelDownloaded() {
      return ReadinessStepSummary(id: "model", label: "Model", detail: "Select", tone: .warning)
    }

    return ReadinessStepSummary(id: "model", label: "Model", detail: "Download", tone: .warning)
  }

  private func threadReadinessStep() -> ReadinessStepSummary {
    guard runtimeState == .ready else {
      return ReadinessStepSummary(id: "thread", label: "Thread", detail: "Waiting", tone: .neutral)
    }
    guard hasRuntimeThreadSelection() else {
      return ReadinessStepSummary(id: "thread", label: "Thread", detail: "Create", tone: .warning)
    }
    if activeTurnID != nil {
      return ReadinessStepSummary(id: "thread", label: "Thread", detail: "Streaming", tone: .active)
    }

    return ReadinessStepSummary(id: "thread", label: "Thread", detail: "Ready", tone: .ready)
  }

  private func appendModelEvent(
    title: String,
    body: String,
    model: LocalModelSummary,
    kind: TimelineEntry.Kind = .system,
    attributes: [String: String] = [:]
  ) {
    var eventAttributes = attributes
    eventAttributes["modelId"] = model.id
    eventAttributes["modelPath"] = model.installPath
    eventAttributes["modelLicense"] = model.license
    appendEntry(
      to: selectedThreadID,
      TimelineEntry(
        id: UUID().uuidString,
        kind: kind,
        title: title,
        body: body,
        attributes: eventAttributes
      )
    )
  }

  private func relaunchRuntimeIfNeeded(runningDetail: String, idleDetail: String) {
    switch runtimeState {
    case .ready:
      runtimeDetail = runningDetail
      runtimeBridge.stopRuntime(detail: runningDetail)
      launchRuntime()
    case .launching:
      runtimeDetail = runningDetail
      runtimeBridge.stopRuntime(detail: runningDetail)
      Task {
        for _ in 0..<10 {
          if runtimeState != .launching {
            break
          }
          try? await Task.sleep(nanoseconds: 200_000_000)
        }
        if runtimeState == .launching {
          runtimeDetail = "Runtime is still launching. Relaunch it after model setup finishes."
          return
        }
        launchRuntime()
      }
    case .disconnected, .failed:
      runtimeDetail = idleDetail
    }
  }

  private func handleRuntimeConnectionStateChange(_ state: RuntimeBridge.ConnectionState, detail: String) {
    let previousState = runtimeState
    runtimeState = state
    runtimeDetail = detail

    switch state {
    case .ready:
      lastRuntimeFailureDetail = nil
    case .failed:
      activeTurnID = nil
      activeTurnThreadID = nil
      modelHealth = nil
      if previousState != .failed || lastRuntimeFailureDetail != detail {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Runtime Disconnected",
            body: "\(detail) Use Relaunch Runtime to recover the local session.",
            attributes: [
              "recovery": "relaunch-runtime"
            ]
          )
        )
      }
      lastRuntimeFailureDetail = detail
    case .disconnected:
      activeTurnID = nil
      activeTurnThreadID = nil
      modelHealth = nil
    case .launching:
      break
    }
  }

  private func writeLocalModelPackManifest(for model: LocalModelSummary) throws -> String {
    let modelURL = URL(fileURLWithPath: model.installPath)
    let manifestURL = modelURL
      .deletingLastPathComponent()
      .appendingPathComponent("model-pack.json")
    let manifest = LocalModelPackManifest(
      id: model.id,
      displayName: model.displayName,
      fileName: model.fileName,
      contextSize: model.contextSize,
      maxOutputTokens: model.maxOutputTokens,
      backend: "llama.cpp",
      license: model.license,
      homepage: model.homepage,
      downloadURL: model.downloadURL,
      sizeBytes: model.sizeBytes
    )
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    let data = try encoder.encode(manifest)
    try FileManager.default.createDirectory(
      at: manifestURL.deletingLastPathComponent(),
      withIntermediateDirectories: true
    )
    try data.write(to: manifestURL, options: .atomic)
    return manifestURL.path
  }

  private func revealSuggestedPath(metricKey: String, successDetail: String) {
    guard let value = modelHealth?.metrics[metricKey], !value.isEmpty else {
      runtimeDetail = "Local model guidance is unavailable until the runtime reports model health."
      return
    }

    let targetURL = URL(fileURLWithPath: value)
    let directoryURL: URL
    var isDirectory = ObjCBool(false)
    if FileManager.default.fileExists(atPath: targetURL.path, isDirectory: &isDirectory) {
      directoryURL = isDirectory.boolValue ? targetURL : targetURL.deletingLastPathComponent()
    } else {
      directoryURL = targetURL.deletingLastPathComponent()
      do {
        try FileManager.default.createDirectory(
          at: directoryURL,
          withIntermediateDirectories: true
        )
      } catch {
        runtimeDetail = "Failed to prepare \(directoryURL.path): \(error.localizedDescription)"
        return
      }
    }

    if NSWorkspace.shared.open(directoryURL) {
      runtimeDetail = successDetail
    } else {
      runtimeDetail = "Failed to open \(directoryURL.path)"
    }
  }

  private func downloadModelFile(
    from sourceURL: URL,
    resumeData: Data?,
    modelID: String,
    expectedBytes: Int64,
    to targetURL: URL
  ) async throws {
    let transfer = ModelDownloadTransfer(targetURL: targetURL) { [weak self] bytesReceived, totalBytes in
      Task { @MainActor [weak self] in
        self?.updateModelDownloadProgress(
          modelID: modelID,
          bytesReceived: bytesReceived,
          totalBytes: totalBytes > 0 ? totalBytes : expectedBytes
        )
      }
    }
    modelDownloadTransfer = transfer
    try await transfer.start(from: sourceURL, resumeData: resumeData)
  }

  private func updateModelDownloadProgress(
    modelID: String,
    bytesReceived: Int64,
    totalBytes: Int64
  ) {
    guard modelDownloadID == modelID,
          var progress = modelDownloadProgress
    else {
      return
    }

    progress.bytesReceived = max(bytesReceived, progress.bytesReceived)
    progress.totalBytes = totalBytes > 0 ? totalBytes : progress.totalBytes
    progress.updatedAt = Date()
    modelDownloadProgress = progress
    runtimeDetail = modelDownloadProgressSummary()
  }

  private func clearPausedModelDownload() {
    pausedModelDownloadID = nil
    modelDownloadResumeData = nil
  }

  private func removeIncompleteModelFile(modelID: String) {
    guard let model = localModels.first(where: { $0.id == modelID }) else {
      return
    }

    let targetURL = URL(fileURLWithPath: model.installPath)
    let manager = FileManager.default
    if manager.fileExists(atPath: targetURL.path) {
      try? manager.removeItem(at: targetURL)
    }
  }

  private func revealFilePath(_ path: String, successDetail: String) {
    guard !path.isEmpty else {
      runtimeDetail = "The requested file path is unavailable."
      return
    }

    let fileURL = URL(fileURLWithPath: path)
    let manager = FileManager.default
    if manager.fileExists(atPath: fileURL.path) {
      NSWorkspace.shared.activateFileViewerSelecting([fileURL])
      runtimeDetail = successDetail
      return
    }

    let parentURL = fileURL.deletingLastPathComponent()
    if manager.fileExists(atPath: parentURL.path) {
      NSWorkspace.shared.activateFileViewerSelecting([parentURL])
      runtimeDetail = "Revealed the closest available folder for \(path)."
    } else {
      runtimeDetail = "Failed to locate \(path)"
    }
  }

  private func storedLastWorkspacePath() -> String? {
    guard let path = UserDefaults.standard.string(forKey: Self.lastWorkspacePathKey),
          !path.isEmpty
    else {
      return nil
    }

    return path
  }

  private func storeLastWorkspacePath(_ path: String) {
    guard !path.isEmpty else {
      return
    }

    UserDefaults.standard.set(path, forKey: Self.lastWorkspacePathKey)
  }

  private func resetWorkspaceSearch() {
    workspaceSearchResults = []
    workspaceSearchStatus = workspace == nil
      ? "Open a workspace before searching."
      : "Search the open workspace by text."
    isWorkspaceSearching = false
  }

  private func isRestorableWorkspacePath(_ path: String) -> Bool {
    var isDirectory = ObjCBool(false)
    return FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory)
      && isDirectory.boolValue
  }

  private func refreshMemoryState() async {
    let runtimeMemoryStatus = try? await runtimeBridge.memoryStatus()
    let runtimeMemoryNotes = try? await runtimeBridge.listMemoryNotes()

    if let runtimeMemoryStatus {
      memoryStatus = MemoryStatusSummary(
        noteCount: runtimeMemoryStatus.noteCount,
        latestTitle: runtimeMemoryStatus.latestTitle,
        summary: runtimeMemoryStatus.summary
      )
    }
    if let runtimeMemoryNotes {
      memoryNotes = runtimeMemoryNotes.map { note in
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
    }
  }

  private func refreshPluginState() async {
    let runtimePlugins = try? await runtimeBridge.listPlugins()
    let runtimeRegistry = try? await runtimeBridge.pluginCapabilityRegistry()
    let runtimeCommands = try? await runtimeBridge.listPluginCommands()
    let runtimeConnectors = try? await runtimeBridge.listPluginConnectors()
    let runtimeHooks = try? await runtimeBridge.listPluginHooks()

    if let runtimePlugins {
      plugins = runtimePlugins.map { pluginSummary(from: $0) }
    }
    if let runtimeRegistry {
      pluginCapabilityRegistrySummary = PluginCapabilityRegistrySummary(
        enabledPluginCount: runtimeRegistry.summary.enabledPluginCount,
        totalCapabilityCount: runtimeRegistry.summary.totalCapabilityCount,
        capabilityCountsByKind: runtimeRegistry.summary.capabilityCountsByKind
      )
      pluginCapabilities = runtimeRegistry.capabilities.map { capability in
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
    } else if runtimePlugins != nil {
      pluginCapabilityRegistrySummary = PluginCapabilityRegistrySummary(
        enabledPluginCount: plugins.filter { $0.status == "ready" && $0.enabled }.count,
        totalCapabilityCount: 0,
        capabilityCountsByKind: [:]
      )
      pluginCapabilities = []
    }
    if let runtimeCommands {
      pluginCommands = runtimeCommands.map { command in
        PluginCommandSummary(
          id: command.commandID,
          title: command.title,
          description: command.description,
          pluginID: command.pluginID,
          pluginDisplayName: command.pluginDisplayName,
          permissions: command.permissions,
          sourcePath: command.sourcePath,
          executionKind: command.executionKind,
          memorySummary: command.memorySummary
        )
      }
    } else if runtimePlugins != nil {
      pluginCommands = []
    }
    if let runtimeConnectors {
      pluginConnectors = runtimeConnectors.map { connector in
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
          credentialStore: connector.credentialStore
        )
      }
    } else if runtimePlugins != nil {
      pluginConnectors = []
    }
    if let runtimeHooks {
      pluginHooks = runtimeHooks.map { hook in
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
    } else if runtimePlugins != nil {
      pluginHooks = []
    }
  }

  private func applyRuntimeThreadUpdate(_ state: RuntimeBridge.RuntimeThreadState) {
    let previousSelectionID = selectedThreadID == state.id ? selectedEntryID : nil
    let entries = timelineEntries(from: state.items)

    threadTimelines[state.id] = entries
    updatePendingApprovals(threadID: state.id, approvals: state.pendingApprovals)
    updateActiveTurn(threadID: state.id, activeTurnID: state.activeTurnID)
    refreshThreadPreview(threadID: state.id, preview: state.status)

    if selectedThreadID == state.id {
      timeline = entries
      if let previousSelectionID,
         entries.contains(where: { $0.id == previousSelectionID }) {
        selectedEntryID = previousSelectionID
      } else {
        selectedEntryID = entries.first?.id
      }
    }
  }

  private func timelineEntries(from items: [RuntimeBridge.RuntimeTimelineItemResult]) -> [TimelineEntry] {
    items.enumerated().map { index, item in
      TimelineEntry(
        id: runtimeTimelineID(for: item, index: index),
        kind: timelineKind(for: item.kind),
        title: item.title,
        body: item.content,
        attributes: item.attributes
      )
    }
  }

  private func runtimeTimelineID(for item: RuntimeBridge.RuntimeTimelineItemResult, index: Int) -> String {
    if let approvalID = item.attributes["approvalId"] {
      return "approval:\(approvalID):\(item.kind):\(item.title)"
    }
    if let turnID = item.attributes["turnId"] {
      return "turn:\(turnID):\(item.kind):\(item.title)"
    }
    return "runtime:\(index):\(item.kind):\(item.title)"
  }

  private func pluginSummary(from plugin: RuntimeBridge.RuntimePlugin) -> PluginSummary {
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

  private func inspectPluginInstallCandidate(at url: URL) throws -> PluginInstallPreview {
    let manifestURL = try pluginManifestURL(for: url)
    let data = try Data(contentsOf: manifestURL)
    let manifest = try JSONDecoder().decode(LocalPluginManifest.self, from: data)
    let installRoot = URL(
      fileURLWithPath: runtimeBridge.localPluginInstallRootPath(),
      isDirectory: true
    )
    let installURL = installRoot.appendingPathComponent(manifest.name, isDirectory: true)

    return PluginInstallPreview(
      sourcePath: url.path,
      manifestPath: manifestURL.path,
      installPath: installURL.path,
      displayName: manifest.displayName,
      version: manifest.version,
      description: manifest.description,
      authorName: manifest.author?.name,
      capabilities: manifest.capabilities,
      permissions: manifest.permissions,
      defaultEnabled: manifest.defaultEnabled
    )
  }

  private func pluginManifestURL(for url: URL) throws -> URL {
    var isDirectory = ObjCBool(false)
    if FileManager.default.fileExists(atPath: url.path, isDirectory: &isDirectory),
       isDirectory.boolValue
    {
      let manifestURL = url.appendingPathComponent("pith-plugin.json", isDirectory: false)
      guard FileManager.default.fileExists(atPath: manifestURL.path) else {
        throw NSError(
          domain: "PithPluginInstall",
          code: 1,
          userInfo: [
            NSLocalizedDescriptionKey:
              "The selected folder does not contain pith-plugin.json."
          ]
        )
      }
      return manifestURL
    }

    guard url.lastPathComponent == "pith-plugin.json" else {
      throw NSError(
        domain: "PithPluginInstall",
        code: 2,
        userInfo: [
          NSLocalizedDescriptionKey:
            "Select a plugin folder or a pith-plugin.json manifest."
        ]
      )
    }

    return url
  }

  private func confirmPluginInstall(preview: PluginInstallPreview) -> Bool {
    let alert = NSAlert()
    alert.alertStyle = .warning
    alert.messageText = "Install Plugin?"
    alert.informativeText = """
      Plugin: \(preview.displayName) \(preview.version)
      Provenance: Local import
      Author: \(preview.authorName ?? "Unknown")
      Source: \(preview.sourcePath)
      Manifest: \(preview.manifestPath)
      Install Path: \(preview.installPath)
      Default Enabled: \(preview.defaultEnabled ? "Yes" : "No")
      Permissions: \(summaryLine(preview.permissions, empty: "none"))
      Capabilities: \(summaryLine(preview.capabilities, empty: "none"))

      \(preview.description)
      """
    alert.addButton(withTitle: "Install")
    alert.addButton(withTitle: "Cancel")
    return alert.runModal() == .alertFirstButtonReturn
  }

  private func confirmModelDownload(
    displayName: String,
    downloadURL: URL,
    targetPath: String,
    sizeSummary: String
  ) -> Bool {
    let alert = NSAlert()
    alert.alertStyle = .informational
    alert.messageText = "Download Local Model?"
    alert.informativeText = """
      Model: \(displayName)
      Size: \(sizeSummary)
      Source: \(downloadURL.absoluteString)
      Target: \(targetPath)

      Pith will store the model locally in app data. The model file is not added to git.
      """
    alert.addButton(withTitle: "Download")
    alert.addButton(withTitle: "Cancel")
    return alert.runModal() == .alertFirstButtonReturn
  }

  private func confirmPluginRemoval(plugin: PluginSummary) -> Bool {
    let alert = NSAlert()
    alert.alertStyle = .warning
    alert.messageText = "Remove Local Plugin?"
    alert.informativeText = """
      Plugin: \(plugin.displayName) \(plugin.version)
      Provenance: \(plugin.provenance)
      Manifest: \(plugin.manifestPath)
      Permissions: \(summaryLine(plugin.permissions, empty: "none"))
      Capabilities: \(summaryLine(plugin.capabilities, empty: "none"))

      Removing this plugin updates the local catalog and can disable related commands, hooks, and permissions.
      """
    alert.addButton(withTitle: "Remove")
    alert.addButton(withTitle: "Cancel")
    return alert.runModal() == .alertFirstButtonReturn
  }

  private func summaryLine(_ values: [String], empty: String) -> String {
    if values.isEmpty {
      return empty
    }

    return values.joined(separator: ", ")
  }

  private func formattedDownloadSize(_ value: String?) -> String {
    guard let value, let byteCount = Int64(value) else {
      return "size unavailable"
    }

    return formattedByteCount(byteCount)
  }

  private func formattedByteCount(_ byteCount: Int64) -> String {
    let formatter = ByteCountFormatter()
    formatter.countStyle = .file
    return formatter.string(fromByteCount: byteCount)
  }

  private func modelDownloadSpeedSummary(_ progress: ModelDownloadProgress) -> String {
    let elapsed = max(progress.updatedAt.timeIntervalSince(progress.startedAt), 1)
    let bytesPerSecond = Int64(Double(progress.bytesReceived) / elapsed)
    return "\(formattedByteCount(bytesPerSecond))/s"
  }

  private static func localModelSummaries(
    storageRootPath: String,
    activeModelPath: String?
  ) -> [LocalModelSummary] {
    let manager = FileManager.default
    let normalizedActivePath = activeModelPath.map { normalizedPath($0) }
    return localModelCatalog().map { item in
      let installPath = item.installPath(storageRootPath: storageRootPath)
      let normalizedInstallPath = normalizedPath(installPath)
      let downloaded = manager.fileExists(atPath: installPath)
      let localSizeBytes = localFileSize(at: installPath)
      return LocalModelSummary(
        id: item.id,
        displayName: item.displayName,
        description: item.description,
        fileName: item.fileName,
        downloadURL: item.downloadURL,
        homepage: item.homepage,
        sizeBytes: item.sizeBytes,
        contextSize: item.contextSize,
        maxOutputTokens: item.maxOutputTokens,
        license: item.license,
        tags: item.tags,
        installPath: installPath,
        downloaded: downloaded,
        active: normalizedActivePath == Optional(normalizedInstallPath),
        localSizeBytes: localSizeBytes
      )
    }
  }

  private static func localFileSize(at path: String) -> Int64? {
    guard let attributes = try? FileManager.default.attributesOfItem(atPath: path),
          let size = attributes[.size] as? NSNumber
    else {
      return nil
    }

    return size.int64Value
  }

  private static func normalizedPath(_ path: String) -> String {
    URL(fileURLWithPath: path).standardizedFileURL.path
  }

  private static func localModelCatalog() -> [LocalModelCatalogItem] {
    [
      LocalModelCatalogItem(
        id: "lfm2.5-350m",
        displayName: "LFM2.5-350M Q4_K_M",
        description: "Default tiny local model for the first Pith agent loop.",
        fileName: "LFM2.5-350M-Q4_K_M.gguf",
        downloadURL: "https://huggingface.co/LiquidAI/LFM2.5-350M-GGUF/resolve/main/LFM2.5-350M-Q4_K_M.gguf",
        homepage: "https://huggingface.co/LiquidAI/LFM2.5-350M-GGUF",
        sizeBytes: 229_312_224,
        contextSize: 4096,
        maxOutputTokens: 160,
        license: "lfm1.0",
        tags: ["default", "tiny", "edge"],
        installSegments: ["builtin", "lfm2.5-350m"]
      ),
      LocalModelCatalogItem(
        id: "qwen2.5-coder-0.5b-instruct",
        displayName: "Qwen2.5-Coder-0.5B Q4_K_M",
        description: "Small code-oriented model for local code generation and repair experiments.",
        fileName: "qwen2.5-coder-0.5b-instruct-q4_k_m.gguf",
        downloadURL: "https://huggingface.co/Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF/resolve/main/qwen2.5-coder-0.5b-instruct-q4_k_m.gguf",
        homepage: "https://huggingface.co/Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF",
        sizeBytes: 491_000_000,
        contextSize: 4096,
        maxOutputTokens: 192,
        license: "apache-2.0",
        tags: ["code", "0.5B", "qwen"],
        installSegments: ["catalog", "qwen2.5-coder-0.5b-instruct"]
      ),
      LocalModelCatalogItem(
        id: "qwen2.5-0.5b-instruct",
        displayName: "Qwen2.5-0.5B Instruct Q4_K_M",
        description: "Compact general chat model with strong multilingual coverage for its size.",
        fileName: "qwen2.5-0.5b-instruct-q4_k_m.gguf",
        downloadURL: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf",
        homepage: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF",
        sizeBytes: 491_000_000,
        contextSize: 4096,
        maxOutputTokens: 192,
        license: "apache-2.0",
        tags: ["chat", "0.5B", "multilingual"],
        installSegments: ["catalog", "qwen2.5-0.5b-instruct"]
      ),
      LocalModelCatalogItem(
        id: "smollm2-360m-instruct",
        displayName: "SmolLM2-360M Q4_K_M",
        description: "Very small instruction model for fast local English assistant experiments.",
        fileName: "SmolLM2-360M-Instruct.Q4_K_M.gguf",
        downloadURL: "https://huggingface.co/QuantFactory/SmolLM2-360M-Instruct-GGUF/resolve/main/SmolLM2-360M-Instruct.Q4_K_M.gguf",
        homepage: "https://huggingface.co/QuantFactory/SmolLM2-360M-Instruct-GGUF",
        sizeBytes: 271_000_000,
        contextSize: 4096,
        maxOutputTokens: 160,
        license: "apache-2.0",
        tags: ["tiny", "english", "fast"],
        installSegments: ["catalog", "smollm2-360m-instruct"]
      ),
    ]
  }

  private func pluginInstallRepairHint(for error: Error) -> String {
    let message = error.localizedDescription

    if message.contains("does not contain pith-plugin.json") {
      return "Choose a plugin folder that contains pith-plugin.json, or select the manifest file directly."
    }

    if message.contains("Select a plugin folder or a pith-plugin.json manifest") {
      return "Point the installer at a plugin directory or the manifest file itself."
    }

    if message.contains("correct format")
      || message.contains("is missing")
    {
      return "Check that pith-plugin.json is valid JSON and uses camelCase keys such as displayName and defaultEnabled."
    }

    return ""
  }
}

private final class ModelDownloadTransfer: NSObject, URLSessionDownloadDelegate {
  private let targetURL: URL
  private let onProgress: (Int64, Int64) -> Void
  private var continuation: CheckedContinuation<Void, Error>?
  private var session: URLSession?
  private var task: URLSessionDownloadTask?
  private var pauseRequested = false

  init(targetURL: URL, onProgress: @escaping (Int64, Int64) -> Void) {
    self.targetURL = targetURL
    self.onProgress = onProgress
  }

  func start(from sourceURL: URL, resumeData: Data?) async throws {
    try await withTaskCancellationHandler {
      try await withCheckedThrowingContinuation { continuation in
        self.continuation = continuation
        let session = URLSession(configuration: .default, delegate: self, delegateQueue: nil)
        self.session = session
        let task = resumeData.map { session.downloadTask(withResumeData: $0) }
          ?? session.downloadTask(with: sourceURL)
        self.task = task
        task.resume()
      }
    } onCancel: {
      self.cancel()
    }
  }

  func pause() {
    pauseRequested = true
    task?.cancel(byProducingResumeData: { [weak self] resumeData in
      guard let self else {
        return
      }

      guard let resumeData, !resumeData.isEmpty else {
        self.complete(.failure(CancellationError()))
        return
      }

      self.complete(.failure(ModelDownloadPaused(resumeData: resumeData)))
    })
  }

  func cancel() {
    pauseRequested = false
    task?.cancel()
  }

  func urlSession(
    _ session: URLSession,
    downloadTask: URLSessionDownloadTask,
    didWriteData _: Int64,
    totalBytesWritten: Int64,
    totalBytesExpectedToWrite: Int64
  ) {
    onProgress(totalBytesWritten, totalBytesExpectedToWrite)
  }

  func urlSession(
    _ session: URLSession,
    downloadTask: URLSessionDownloadTask,
    didFinishDownloadingTo location: URL
  ) {
    if let httpResponse = downloadTask.response as? HTTPURLResponse,
       !(200..<300).contains(httpResponse.statusCode)
    {
      complete(
        .failure(
          NSError(
            domain: "PithModelDownload",
            code: httpResponse.statusCode,
            userInfo: [
              NSLocalizedDescriptionKey:
                "Model download failed with HTTP \(httpResponse.statusCode)."
            ]
          )
        )
      )
      return
    }

    do {
      let manager = FileManager.default
      try manager.createDirectory(
        at: targetURL.deletingLastPathComponent(),
        withIntermediateDirectories: true
      )

      if manager.fileExists(atPath: targetURL.path) {
        try manager.removeItem(at: targetURL)
      }

      try manager.moveItem(at: location, to: targetURL)
      complete(.success(()))
    } catch {
      complete(.failure(error))
    }
  }

  func urlSession(
    _ session: URLSession,
    task: URLSessionTask,
    didCompleteWithError error: Error?
  ) {
    guard let error else {
      return
    }

    if pauseRequested {
      if let resumeData = (error as NSError).userInfo[NSURLSessionDownloadTaskResumeData] as? Data,
         !resumeData.isEmpty
      {
        complete(.failure(ModelDownloadPaused(resumeData: resumeData)))
      }
      return
    }

    complete(.failure(error))
  }

  private func complete(_ result: Result<Void, Error>) {
    guard let continuation else {
      return
    }

    self.continuation = nil
    task = nil
    session?.invalidateAndCancel()
    session = nil

    switch result {
    case .success:
      continuation.resume()
    case .failure(let error):
      continuation.resume(throwing: error)
    }
  }
}
