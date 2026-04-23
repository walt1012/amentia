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

  init(runtimeBridge: RuntimeBridge = RuntimeBridge()) {
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
    self.timeline = [
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
        } else {
          threads = threadList.map { ThreadSummary(id: $0.id, title: $0.title, preview: $0.status) }
        }

        selectedThreadID = threads.first?.id
        timeline.insert(
          TimelineEntry(
            id: UUID(),
            kind: .system,
            title: "Runtime Connected",
            body: "Connected to \(session.serverName) \(session.serverVersion) over stdio."
          ),
          at: 0
        )
      } catch {
        runtimeState = .failed
        runtimeDetail = error.localizedDescription
        timeline.insert(
          TimelineEntry(
            id: UUID(),
            kind: .system,
            title: "Runtime Launch Failed",
            body: error.localizedDescription
          ),
          at: 0
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
        selectedThreadID = thread.id
        timeline.insert(
          TimelineEntry(
            id: UUID(),
            kind: .system,
            title: "Thread Created",
            body: "Created \(thread.title) in the local runtime."
          ),
          at: 0
        )
      } catch {
        timeline.insert(
          TimelineEntry(
            id: UUID(),
            kind: .system,
            title: "Thread Creation Failed",
            body: error.localizedDescription
          ),
          at: 0
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
        appendMessagesToTimeline(result.messages)
        refreshThreadPreview(threadID: threadID, preview: "\(result.turnID) ready")
      } catch {
        timeline.insert(
          TimelineEntry(
            id: UUID(),
            kind: .system,
            title: "Turn Failed",
            body: error.localizedDescription
          ),
          at: 0
        )
      }
    }
  }

  private func appendMessagesToTimeline(_ messages: [RuntimeBridge.RuntimeTurnMessageResult]) {
    let newEntries = messages.map { message in
      TimelineEntry(
        id: UUID(),
        kind: message.role == "user" ? .userMessage : .assistantMessage,
        title: message.role == "user" ? "User" : "Assistant",
        body: message.content
      )
    }

    timeline.insert(contentsOf: newEntries.reversed(), at: 0)
  }

  private func refreshThreadPreview(threadID: String, preview: String) {
    guard let index = threads.firstIndex(where: { $0.id == threadID }) else {
      return
    }

    threads[index].preview = preview
  }
}
