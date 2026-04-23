import AppKit
import Foundation

@MainActor
final class AppViewModel: ObservableObject {
  @Published var threads: [ThreadSummary]
  @Published var selectedThreadID: ThreadSummary.ID?
  @Published var timeline: [TimelineEntry]
  @Published var runtimeState: RuntimeBridge.ConnectionState
  @Published var runtimeDetail: String
  @Published var draftMessage: String
  @Published var workspace: WorkspaceSummary?

  private let runtimeBridge: RuntimeBridge
  private var threadTimelines: [String: [TimelineEntry]]

  init(runtimeBridge: RuntimeBridge = RuntimeBridge()) {
    let initialTimeline = [
      TimelineEntry(
        id: UUID(),
        kind: .system,
        title: "Milestone 1 Start",
        body: "Launch the runtime, open a workspace, and ask Cavell to inspect local files."
      ),
      TimelineEntry(
        id: UUID(),
        kind: .assistantMessage,
        title: "Next Step",
        body: "The first local agent loop uses workspace-aware read, list, and search tools before approvals and writes land."
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
    self.threads = initialThreads
    self.timeline = initialTimeline
    self.threadTimelines = ["local-welcome": initialTimeline]
    self.selectedThreadID = initialThreads.first?.id
  }

  func launchRuntime() {
    runtimeState = .launching
    runtimeDetail = "Launching local runtime"

    Task {
      do {
        let session = try await runtimeBridge.launchAndInitialize()
        let threadList = try await runtimeBridge.listThreads()

        runtimeState = .ready
        runtimeDetail = "\(session.serverName) \(session.serverVersion)"

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
            body: "Connected to \(session.serverName) \(session.serverVersion) over stdio."
          )
        )
      } catch {
        runtimeState = .failed
        runtimeDetail = error.localizedDescription
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID(),
            kind: .warning,
            title: "Runtime Launch Failed",
            body: error.localizedDescription
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
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID(),
            kind: .system,
            title: "Workspace Opened",
            body: "Opened \(openedWorkspace.displayName) at \(openedWorkspace.rootPath)."
          )
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID(),
            kind: .warning,
            title: "Workspace Open Failed",
            body: error.localizedDescription
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
        selectThread(id: thread.id)
        await loadThreadHistory(threadID: thread.id)
        appendEntry(
          to: thread.id,
          TimelineEntry(
            id: UUID(),
            kind: .system,
            title: "Thread Created",
            body: "Created \(thread.title) in the local runtime."
          )
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID(),
            kind: .warning,
            title: "Thread Creation Failed",
            body: error.localizedDescription
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
          workspace != nil
    else {
      return
    }

    draftMessage = ""

    Task {
      do {
        let result = try await runtimeBridge.startTurn(threadID: threadID, message: message)
        appendItemsToTimeline(threadID: result.threadID, items: result.items)
        refreshThreadPreview(threadID: result.threadID, preview: "\(result.turnID) ready")
      } catch {
        appendEntry(
          to: threadID,
          TimelineEntry(
            id: UUID(),
            kind: .warning,
            title: "Turn Failed",
            body: error.localizedDescription
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

  func workspaceDisplayName() -> String {
    workspace?.displayName ?? "No Workspace"
  }

  func workspacePath() -> String {
    workspace?.rootPath ?? "Open a local workspace to enable Milestone 1 tools."
  }

  func composerPlaceholder() -> String {
    if workspace == nil {
      return "Open a workspace to start local agent work"
    }

    return "Ask Cavell to inspect or search files in the current workspace"
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
        body: item.content
      )
    }

    for entry in newEntries.reversed() {
      appendEntry(to: threadID, entry)
    }
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
      return
    }

    var entries = threadTimelines[threadID] ?? defaultTimeline(for: threadTitle(for: threadID))
    entries.insert(entry, at: 0)
    threadTimelines[threadID] = entries

    if selectedThreadID == threadID {
      timeline = entries
    }
  }

  private func syncVisibleTimeline() {
    guard let selectedThreadID else {
      timeline = []
      return
    }

    timeline = threadTimelines[selectedThreadID] ?? defaultTimeline(for: threadTitle(for: selectedThreadID))
    threadTimelines[selectedThreadID] = timeline
  }

  private func defaultTimeline(for title: String) -> [TimelineEntry] {
    [
      TimelineEntry(
        id: UUID(),
        kind: .system,
        title: "Thread Ready",
        body: "\(title) is ready for local runtime messages."
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
          body: item.content
        )
      }
      threadTimelines[threadID] = entries
      refreshThreadPreview(threadID: threadID, preview: result.status)

      if selectedThreadID == threadID {
        timeline = entries
      }
    } catch {
      appendEntry(
        to: threadID,
        TimelineEntry(
          id: UUID(),
          kind: .warning,
          title: "Thread Load Failed",
          body: error.localizedDescription
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
    case "toolStart", "toolResult":
      return .tool
    case "warning":
      return .warning
    default:
      return .system
    }
  }
}
