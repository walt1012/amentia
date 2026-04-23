import Foundation

@MainActor
final class AppViewModel: ObservableObject {
  @Published var threads: [ThreadSummary]
  @Published var selectedThreadID: ThreadSummary.ID?
  @Published var timeline: [TimelineEntry]
  @Published var runtimeState: RuntimeBridge.ConnectionState
  @Published var runtimeDetail: String

  private let runtimeBridge: RuntimeBridge

  init(runtimeBridge: RuntimeBridge = RuntimeBridge()) {
    self.runtimeBridge = runtimeBridge
    self.runtimeState = runtimeBridge.connectionState
    self.runtimeDetail = "Runtime not launched"
    self.threads = [
      ThreadSummary(
        id: UUID(),
        title: "Welcome to Cavell",
        preview: "Milestone 0 shell ready for runtime integration."
      ),
    ]
    self.selectedThreadID = threads.first?.id
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
          threads = [
            ThreadSummary(
              id: UUID(),
              title: "New Thread",
              preview: "Runtime connected and ready for the first real conversation."
            ),
          ]
        } else {
          threads = threadList.map {
            ThreadSummary(
              id: UUID(uuidString: $0.id) ?? UUID(),
              title: $0.title,
              preview: $0.status
            )
          }
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
}
