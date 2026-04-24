import AppKit
import Foundation

@MainActor
final class AppViewModel: ObservableObject {
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
  @Published var pluginHooks: [PluginHookSummary]

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
    self.pluginHooks = []
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
        if !pluginHooks.isEmpty {
          runtimeDetail += " | \(pluginHooks.count) hook(s)"
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
      } catch {
        runtimeState = .failed
        runtimeDetail = error.localizedDescription
        modelHealth = nil
        memoryStatus = nil
        memoryNotes = []
        plugins = []
        pluginCapabilityRegistrySummary = nil
        pluginCapabilities = []
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
          isLocalModelReady(),
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

  func canDownloadLocalModel() -> Bool {
    guard runtimeState == .ready,
          let modelHealth,
          let downloadURL = modelHealth.metrics["downloadUrl"],
          let targetPath = modelHealth.metrics["suggestedModelPath"]
    else {
      return false
    }

    return !downloadURL.isEmpty && !targetPath.isEmpty && !isLocalModelReady()
  }

  func downloadLocalModel() {
    guard runtimeState == .ready else {
      runtimeDetail = "Launch the runtime before downloading the local model."
      return
    }

    guard let modelHealth else {
      runtimeDetail = "Local model guidance is unavailable until the runtime reports model health."
      return
    }

    guard let downloadURLValue = modelHealth.metrics["downloadUrl"],
          let downloadURL = URL(string: downloadURLValue),
          !downloadURLValue.isEmpty
    else {
      runtimeDetail = "The local model pack does not include a download URL yet."
      return
    }

    guard let targetPath = modelHealth.metrics["suggestedModelPath"], !targetPath.isEmpty else {
      runtimeDetail = "The local model target path is unavailable."
      return
    }

    let sizeSummary = formattedDownloadSize(modelHealth.metrics["sizeBytes"])
    guard confirmModelDownload(
      displayName: modelHealth.displayName,
      downloadURL: downloadURL,
      targetPath: targetPath,
      sizeSummary: sizeSummary
    ) else {
      runtimeDetail = "Local model download was cancelled."
      return
    }

    Task {
      do {
        runtimeDetail = "Preparing local model download..."
        let bootstrap = try await runtimeBridge.bootstrapModelPack()
        let targetURL = URL(fileURLWithPath: targetPath)
        if FileManager.default.fileExists(atPath: targetURL.path) {
          await refreshModelHealthState()
          runtimeDetail = "Local model already exists at \(targetURL.path). Manifest: \(bootstrap.manifestPath)"
          return
        }

        runtimeDetail = "Downloading \(modelHealth.displayName) (\(sizeSummary))..."
        try await downloadModelFile(from: downloadURL, to: targetURL)
        let refreshedBootstrap = try await runtimeBridge.bootstrapModelPack()
        await refreshModelHealthState()
        runtimeDetail = "Downloaded \(modelHealth.displayName) to \(targetURL.path). Manifest: \(refreshedBootstrap.manifestPath)"
        appendEntry(
          to: selectedThread?.id,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: "Local Model Downloaded",
            body: "\(modelHealth.displayName) was downloaded to \(targetURL.path).",
            timestamp: Date(),
            attributes: [
              "modelPath": targetURL.path,
              "source": downloadURL.absoluteString,
            ]
          )
        )
      } catch {
        runtimeDetail = "Model download failed: \(error.localizedDescription)"
      }
    }
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
    if workspace == nil {
      return "Open a workspace to start local agent work"
    }

    if !isLocalModelReady() {
      return "Install the local LFM2.5-350M runtime before starting agent work"
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
    case "toolStart", "toolResult", "pluginCommand":
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

  private nonisolated func downloadModelFile(from sourceURL: URL, to targetURL: URL) async throws {
    let data = try Data(contentsOf: sourceURL)
    let manager = FileManager.default
    try manager.createDirectory(
      at: targetURL.deletingLastPathComponent(),
      withIntermediateDirectories: true
    )

    if manager.fileExists(atPath: targetURL.path) {
      try manager.removeItem(at: targetURL)
    }

    try data.write(to: targetURL, options: .atomic)
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
          sourcePath: command.sourcePath,
          memorySummary: command.memorySummary
        )
      }
    } else if runtimePlugins != nil {
      pluginCommands = []
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

    let formatter = ByteCountFormatter()
    formatter.countStyle = .file
    return formatter.string(fromByteCount: byteCount)
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
