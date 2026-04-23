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
  @Published var plugins: [PluginSummary]

  private let runtimeBridge: RuntimeBridge
  private var threadTimelines: [String: [TimelineEntry]]
  private var threadPendingApprovalIDs: [String: Set<String>]
  private var activeTurnThreadID: String?

  init(runtimeBridge: RuntimeBridge = RuntimeBridge()) {
    let initialTimeline = [
      TimelineEntry(
        id: UUID(),
        kind: .system,
        title: "Milestone 1 Start",
        body: "Launch the runtime, open a workspace, and ask Cavell to inspect local files.",
        attributes: [:]
      ),
      TimelineEntry(
        id: UUID(),
        kind: .assistantMessage,
        title: "Next Step",
        body:
          "The first local agent loop uses workspace-aware read, search, shell, and approval-gated write tools.",
        attributes: [:]
      ),
    ]

    let initialThreads = [
      ThreadSummary(
        id: "local-welcome",
        title: "Welcome to Cavell",
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
    self.plugins = []
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
  }

  func launchRuntime() {
    runtimeState = .launching
    runtimeDetail = "Launching local runtime"

    Task {
      do {
        let session = try await runtimeBridge.launchAndInitialize()
        let runtimeModel = try? await runtimeBridge.modelHealth()
        let runtimeMemoryStatus = try? await runtimeBridge.memoryStatus()
        let runtimeMemoryNotes = try? await runtimeBridge.listMemoryNotes()
        let currentWorkspace = try? await runtimeBridge.currentWorkspace()
        let runtimePlugins = try? await runtimeBridge.listPlugins()
        let threadList = try await runtimeBridge.listThreads()

        runtimeState = .ready
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
          runtimeDetail =
            "\(session.serverName) \(session.serverVersion) | \(runtimeModel.displayName)"
        } else {
          modelHealth = nil
          runtimeDetail = "\(session.serverName) \(session.serverVersion)"
        }

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

        plugins = (runtimePlugins ?? []).map { plugin in
          PluginSummary(
            id: plugin.id,
            name: plugin.name,
            version: plugin.version,
            displayName: plugin.displayName,
            description: plugin.description,
            authorName: plugin.authorName,
            enabled: plugin.enabled,
            defaultEnabled: plugin.defaultEnabled,
            capabilities: plugin.capabilities,
            permissions: plugin.permissions,
            manifestPath: plugin.manifestPath,
            provenance: plugin.provenance
          )
        }
        if !plugins.isEmpty {
          runtimeDetail += " | \(plugins.count) plugin(s)"
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
            id: UUID(),
            kind: .system,
            title: "Runtime Connected",
            body: "Connected to \(session.serverName) \(session.serverVersion) over stdio.",
            attributes: [:]
          )
        )
        if let runtimeModel {
          appendEntry(
            to: selectedThread?.id,
            TimelineEntry(
              id: UUID(),
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
              id: UUID(),
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
              id: UUID(),
              kind: .system,
              title: "Plugins Ready",
              body: "Discovered \(plugins.count) plugin(s): \(plugins.map(\.displayName).joined(separator: ", ")).",
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
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID(),
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
    panel.message = "Choose a local workspace for Cavell to inspect."

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
            id: UUID(),
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
            id: UUID(),
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
            id: UUID(),
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
            id: UUID(),
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
          workspace != nil,
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
            id: UUID(),
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
            id: UUID(),
            kind: .warning,
            title: "Approval Response Failed",
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
            id: UUID(),
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
      return "Cavell will use the built-in local model path after the runtime connects."
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

  func modelArtifactPathSummary() -> String {
    guard let modelHealth else {
      return "No local model paths available yet."
    }

    let modelPath = modelHealth.modelPath ?? "model path unavailable"
    let binaryPath = modelHealth.binaryPath ?? "binary path unavailable"
    let manifestPath = modelHealth.manifestPath ?? "manifest path unavailable"
    return "Model: \(modelPath)\nBinary: \(binaryPath)\nManifest: \(manifestPath)"
  }

  func pluginCountSummary() -> String {
    if plugins.isEmpty {
      return "No bundled plugins discovered yet."
    }

    return "\(plugins.count) plugin(s) discovered"
  }

  func pluginDetailSummary() -> String {
    guard !plugins.isEmpty else {
      return "Cavell discovers plugin manifests from the bundled plugins directory."
    }

    return plugins
      .map { plugin in
        let capabilities = plugin.capabilities.isEmpty ? "none" : plugin.capabilities.joined(separator: ", ")
        return "\(plugin.displayName) \(plugin.version) | \(plugin.provenance) | capabilities: \(capabilities)"
      }
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
      return "Cavell uses built-in memory instead of a memory plugin. Workspace notes are stored locally by the runtime."
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

  func composerPlaceholder() -> String {
    if workspace == nil {
      return "Open a workspace to start local agent work"
    }

    if activeTurnID != nil {
      return "Cavell is streaming a response. Cancel to stop the current turn."
    }

    return "Ask Cavell to inspect files, review diffs, run shell commands, or write files"
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
        id: UUID(),
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
        id: UUID(),
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
      let entries = result.items.map { item in
        TimelineEntry(
          id: UUID(),
          kind: timelineKind(for: item.kind),
          title: item.title,
          body: item.content,
          attributes: item.attributes
        )
      }
      threadTimelines[threadID] = entries
      updatePendingApprovals(threadID: threadID, approvals: result.pendingApprovals)
      updateActiveTurn(threadID: threadID, activeTurnID: result.activeTurnID)
      refreshThreadPreview(threadID: threadID, preview: result.status)

      if selectedThreadID == threadID {
        timeline = entries
        selectedEntryID = entries.first?.id
      }
    } catch {
      appendEntry(
        to: threadID,
        TimelineEntry(
          id: UUID(),
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

  private func applyRuntimeThreadUpdate(_ state: RuntimeBridge.RuntimeThreadState) {
    let entries = state.items.map { item in
      TimelineEntry(
        id: UUID(),
        kind: timelineKind(for: item.kind),
        title: item.title,
        body: item.content,
        attributes: item.attributes
      )
    }

    threadTimelines[state.id] = entries
    updatePendingApprovals(threadID: state.id, approvals: state.pendingApprovals)
    updateActiveTurn(threadID: state.id, activeTurnID: state.activeTurnID)
    refreshThreadPreview(threadID: state.id, preview: state.status)

    if selectedThreadID == state.id {
      timeline = entries
      selectedEntryID = entries.first?.id
    }
  }
}
