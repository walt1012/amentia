import Darwin
import Foundation

final class RuntimeBridgeProcessSession {
  let identifier: ObjectIdentifier
  let inputHandle: FileHandle

  private let process: Process
  private let outputHandle: FileHandle
  private let errorHandle: FileHandle
  private let recentErrorOutputQueue = DispatchQueue(label: "amentia.runtime.bridge.stderr-tail")
  private let recentErrorOutputLimit = 4096
  private let recentErrorSummaryLimit = 900
  private let terminationGraceSeconds = 2.0
  private let terminationPollSeconds = 0.05
  private var recentErrorOutput = ""
  private var readerTask: Task<Void, Never>?
  private var errorReaderTask: Task<Void, Never>?

  var isRunning: Bool {
    process.isRunning
  }

  var recentErrorSummary: String? {
    recentErrorOutputQueue.sync {
      let summary = Self.compactRuntimeErrorOutput(
        recentErrorOutput,
        limit: recentErrorSummaryLimit
      )
      return summary.isEmpty ? nil : summary
    }
  }

  init(
    executableURL: URL,
    environment: [String: String]
  ) throws {
    let process = Process()
    let stdinPipe = Pipe()
    let stdoutPipe = Pipe()
    let stderrPipe = Pipe()

    process.executableURL = executableURL
    process.arguments = []
    process.environment = environment
    process.standardInput = stdinPipe
    process.standardOutput = stdoutPipe
    process.standardError = stderrPipe

    let processIdentifier = ObjectIdentifier(process)
    self.process = process
    self.identifier = processIdentifier
    self.inputHandle = stdinPipe.fileHandleForWriting
    self.outputHandle = stdoutPipe.fileHandleForReading
    self.errorHandle = stderrPipe.fileHandleForReading

    try process.run()
  }

  func startObserving(
    onLine: @escaping @Sendable (ObjectIdentifier, Data) -> Void,
    onReadError: @escaping @Sendable (ObjectIdentifier, Error) -> Void,
    onTermination: @escaping @Sendable (ObjectIdentifier, String) -> Void
  ) {
    let processIdentifier = identifier
    process.terminationHandler = { [processIdentifier] process in
      let detail = "Amentia exited with status \(process.terminationStatus)."
      onTermination(processIdentifier, detail)
    }

    guard process.isRunning else {
      onTermination(processIdentifier, "Amentia exited with status \(process.terminationStatus).")
      return
    }

    startReaderLoop(
      with: outputHandle,
      onLine: onLine,
      onReadError: onReadError
    )
    startErrorReaderLoop(with: errorHandle)
  }

  func stop() {
    readerTask?.cancel()
    readerTask = nil
    errorReaderTask?.cancel()
    errorReaderTask = nil
    process.terminationHandler = nil

    try? inputHandle.close()

    if process.isRunning {
      process.terminate()
      waitForTerminationOrKill()
    }

    try? outputHandle.close()
    try? errorHandle.close()
  }

  private func waitForTerminationOrKill() {
    let deadline = Date().addingTimeInterval(terminationGraceSeconds)
    while process.isRunning && Date() < deadline {
      Thread.sleep(forTimeInterval: terminationPollSeconds)
    }
    guard process.isRunning else {
      return
    }

    Darwin.kill(process.processIdentifier, SIGKILL)
    process.waitUntilExit()
  }

  private func startReaderLoop(
    with handle: FileHandle,
    onLine: @escaping @Sendable (ObjectIdentifier, Data) -> Void,
    onReadError: @escaping @Sendable (ObjectIdentifier, Error) -> Void
  ) {
    let processIdentifier = identifier
    readerTask = Task.detached(priority: .userInitiated) {
      while !Task.isCancelled {
        let line: String
        do {
          line = try Self.readLine(from: handle)
        } catch {
          onReadError(processIdentifier, error)
          return
        }

        onLine(processIdentifier, Data(line.utf8))
      }
    }
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
      throw RuntimeBridge.RuntimeError.invalidResponse
    }

    return String(decoding: data, as: UTF8.self)
  }

  private func startErrorReaderLoop(with handle: FileHandle) {
    errorReaderTask = Task.detached(priority: .utility) {
      while !Task.isCancelled {
        do {
          let chunk = try handle.read(upToCount: 4096) ?? Data()
          if chunk.isEmpty {
            return
          }

          self.appendRecentErrorOutput(chunk)

          #if DEBUG
            if let rawMessage = String(data: chunk, encoding: .utf8) {
              let message = rawMessage.trimmingCharacters(in: .whitespacesAndNewlines)
              guard !message.isEmpty else {
                continue
              }
              print("[amentia-runtime stderr] \(message)")
            }
          #endif
        } catch {
          return
        }
      }
    }
  }

  private func appendRecentErrorOutput(_ chunk: Data) {
    guard let message = String(data: chunk, encoding: .utf8) else {
      return
    }

    recentErrorOutputQueue.sync {
      recentErrorOutput += message
      if recentErrorOutput.count > recentErrorOutputLimit {
        recentErrorOutput = String(recentErrorOutput.suffix(recentErrorOutputLimit))
      }
    }
  }

  private static func compactRuntimeErrorOutput(_ output: String, limit: Int) -> String {
    let normalized = output
      .components(separatedBy: .whitespacesAndNewlines)
      .filter { !$0.isEmpty }
      .joined(separator: " ")
      .trimmingCharacters(in: .whitespacesAndNewlines)

    guard normalized.count > limit else {
      return normalized
    }

    return "..." + String(normalized.suffix(limit))
  }
}
