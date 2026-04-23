import Foundation

@MainActor
final class AppViewModel: ObservableObject {
  @Published var threads: [ThreadSummary]
  @Published var selectedThreadID: ThreadSummary.ID?
  @Published var timeline: [TimelineEntry]
  @Published var runtimeState: RuntimeBridge.ConnectionState
  @Published var runtimeDetail: String
  @Published var draftMessage: String

  private let runtimeBridge: RuntimeBridge
  private var threadTimelines: [String: [TimelineEntry]]

  init(runtimeBridge: RuntimeBridge = RuntimeBridge()) {
    let initialTimeline = [
      TimelineEntry(
        id: UUID(),
        kind: .system,
        title: "Runtime Boundary",
        body: "The macOS shell is prepared to talk to a local Rust runtime over stdio."
      ),
      TimelineEntry(
        id: UUID(),
        kind: .assistantMessage,
        title: "Next Step",
        body: "Connect the real runtime handshake and replace mock state with live protocol events."
      ),
    ]

    let initialThreads = [
      ThreadSummary(
        id: "local-welcome",
        title: "Welcome to Cavell",
        preview: "Milestone 0 shell ready for runtime integration."
      ),
    ]

    self.runtimeBridge = runtimeBridge
    self.runtimeState = runtimeBridge.connectionState
    self.runtimeDetail = "Runtime not launched"
    self.draftMessage = ""
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
          let firstThread = try await runtimeBridge.startThread(title: "First Thread")
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
        appendEntry(
          to: selectedThread?.id,
          TimelineEntry(
            id: UUID(),
            kind: .system,
            title: "Runtime Connected",
            body: "Connected to \(session.serverName) \(session.serverVersion) over stdio."
          ),
        )
      } catch {
        runtimeState = .failed
        runtimeDetail = error.localizedDescription
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID(),
            kind: .system,
            title: "Runtime Launch Failed",
            body: error.localizedDescription
          ),
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
        appendEntry(
          to: thread.id,
          TimelineEntry(
            id: UUID(),
            kind: .system,
            title: "Thread Created",
            body: "Created \(thread.title) in the local runtime."
          ),
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID(),
            kind: .system,
            title: "Thread Creation Failed",
            body: error.localizedDescription
          ),
        )
      }
    }
  }

  func sendDraftMessage() {
    let message = draftMessage.trimmingCharacters(in: .whitespacesAndNewlines)

    guard runtimeState == .ready, !message.isEmpty, let threadID = selectedThreadID else {
      return
    }

    draftMessage = ""

    Task {
      do {
        let result = try await runtimeBridge.startTurn(threadID: threadID, message: message)
        appendMessagesToTimeline(threadID: result.threadID, messages: result.messages)
        refreshThreadPreview(threadID: result.threadID, preview: "\(result.turnID) ready")
      } catch {
        appendEntry(
          to: threadID,
          TimelineEntry(
            id: UUID(),
            kind: .system,
            title: "Turn Failed",
            body: error.localizedDescription
          ),
        )
      }
    }
  }

  func selectThread(id: String?) {
    selectedThreadID = id
    syncVisibleTimeline()
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

  private func appendMessagesToTimeline(
    threadID: String,
    messages: [RuntimeBridge.RuntimeTurnMessageResult]
  ) {
    let newEntries = messages.map { message in
      TimelineEntry(
        id: UUID(),
        kind: message.role == "user" ? .userMessage : .assistantMessage,
        title: message.role == "user" ? "User" : "Assistant",
        body: message.content
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
}
