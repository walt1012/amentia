import AppKit
import Foundation

@MainActor
final class AppViewModel: ObservableObject {
  @Published var threads: [ThreadSummary]
  @Published var selectedThreadID: ThreadSummary.ID?
  @Published var timeline: [TimelineEntry]
  @Published var selectedEntryID: TimelineEntry.ID?
  @Published var activeTurnID: String?
  @Published var runtimeState: RuntimeBridge.ConnectionState
  @Published var runtimeDetail: String
  @Published var draftMessage: String
  @Published var workspace: WorkspaceSummary?
  @Published var modelHealth: ModelHealthSummary?
  @Published var memoryStatus: MemoryStatusSummary?
  @Published var memoryNotes: [MemoryNoteSummary]
  @Published var memoryNoteTitle: String
  @Published var memoryNoteBody: String
  @Published var plugins: [PluginSummary]
  @Published var pluginCapabilityRegistrySummary: PluginCapabilityRegistrySummary?
  @Published var pluginCapabilities: [PluginCapabilitySummary]
  @Published var pluginCommands: [PluginCommandSummary]

  private let runtimeBridge: RuntimeBridge
  private var threadTimelines: [String: [TimelineEntry]]
  private var threadPendingApprovalIDs: [String: Set<String>]
  private var activeTurnThreadID: String?

  init(runtimeBridge: RuntimeBridge = RuntimeBridge()) {
    let initialTimeline = [
      TimelineEntry(
        id: UUID().uuidString,
        kind: .system,
        title: "Milestone 1 Ready",
        body: "Open a workspace, launch the runtime, and ask Pith to inspect or change local files.",
        attributes: [:]
      ),
      TimelineEntry(
        id: UUID().uuidString,
        kind: .assistantMessage,
        title: "Local Agent Loop",
        body:
          "Pith now supports workspace-aware read, search, shell, diff, memory, and approval-gated write actions.",
        attributes: [:]
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
    self.modelHealth = nil
    self.memoryStatus = nil
    self.memoryNotes = []
    self.memoryNoteTitle = ""
    self.memoryNoteBody = ""
    self.plugins = []
    self.pluginCapabilityRegistrySummary = nil
    self.pluginCapabilities = []
    self.pluginCommands = []
    self.threads = initialThreads
    self.timeline = initialTimeline
    self.selectedEntryID = initialTimeline.first?.id
    self.activeTurnID = nil
    self.threadTimelines = ["local-welcome": initialTimeline]
    self.threadPendingApprovalIDs = [:]
    self.selectedThreadID = initialThreads.first?.id
    self.runtimeBridge.onThreadUpdated = { [weak self] state in
      Task { @MainActor in
        self?.applyRuntimeThreadUpdate(state)
      }
    }
    self.runtimeBridge.onConnectionStateChanged = { [weak self] state, detail in
      Task { @MainActor in
        self?.runtimeState = state
        self?.runtimeDetail = detail
      }
    }
  }

  func launchRuntime() {
    runtimeState = .launching
    runtimeDetail = "Launching local runtime"

    Task {
      do {
        let session = try await runtimeBridge.launchAndInitialize()
        let runtimeMemoryStatus = try? await runtimeBridge.memoryStatus()
        let runtimeMemoryNotes = try? await runtimeBridge.listMemoryNotes()
        let currentWorkspace = try? await runtimeBridge.currentWorkspace()
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

        if let currentWorkspace {
          workspace = WorkspaceSummary(
            rootPath: currentWorkspace.rootPath,
            displayName: currentWorkspace.displayName
          )
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
        if let runtimeModel = modelHealth {
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
      } catch {
        runtimeState = .failed
        runtimeDetail = error.localizedDescription
        modelHealth = nil
        memoryStatus = nil
        memoryNotes = []
        plugins = []
        pluginCapabilityRegistrySummary = nil
        pluginCapabilities = []
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
          !message.isEmpty,
          let threadID = selectedThreadID,
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

  func runPluginCommand(commandID: String) {
    guard runtimeState == .ready,
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

  func selectedDiffBody() -> String? {
    guard let entry = selectedEntry(), entry.kind == .diff else {
      return nil
    }

    return entry.body
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

  func pluginDetailSummary() -> String {
    guard !plugins.isEmpty else {
      return "Pith discovers plugin manifests from the bundled plugins directory."
    }

    return plugins
      .map { plugin in
        let capabilities = plugin.capabilities.isEmpty ? "none" : plugin.capabilities.joined(separator: ", ")
        let validation = plugin.validationError ?? "ok"
        return "\(plugin.displayName) \(plugin.version) | \(plugin.status) | \(plugin.provenance) | capabilities: \(capabilities) | validation: \(validation)"
      }
      .joined(separator: "\n")
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

  func composerPlaceholder() -> String {
    if workspace == nil {
      return "Open a workspace to start local agent work"
    }

    if activeTurnID != nil {
      return "Pith is streaming a response. Cancel to stop the current turn."
    }

    return "Ask Pith to inspect files, review diffs, run shell commands, or write files"
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
        body: "\(title) is ready for local runtime messages.",
        attributes: [:]
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
    case "toolStart", "toolResult":
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
    } else {
      modelHealth = nil
      if let serverLabel {
        runtimeDetail = serverLabel
      }
    }
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
          manifestPath: capability.manifestPath
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
          sourcePath: command.sourcePath
        )
      }
    } else if runtimePlugins != nil {
      pluginCommands = []
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
      validationError: plugin.validationError
    )
  }
}
