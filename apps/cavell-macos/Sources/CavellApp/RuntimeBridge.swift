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
  }

  struct RuntimeWorkspace {
    let rootPath: String
    let displayName: String
    let threadCount: Int
  }

  struct RuntimeTurnResult {
    let turnID: String
    let threadID: String
    let items: [RuntimeTimelineItemResult]
  }

  struct RuntimeThreadState {
    let id: String
    let title: String
    let status: String
    let items: [RuntimeTimelineItemResult]
  }

  struct RuntimeTimelineItemResult {
    let kind: String
    let title: String
    let content: String
  }

  enum RuntimeError: LocalizedError {
    case runtimePathMissing
    case runtimePipeUnavailable
    case invalidResponse
    case rpc(String)

    var errorDescription: String? {
      switch self {
      case .runtimePathMissing:
        return "The runtime binary could not be found. Set CAVELL_RUNTIME_PATH to the built runtime executable."
      case .runtimePipeUnavailable:
        return "The runtime process pipes are not available."
      case .invalidResponse:
        return "The runtime returned an invalid response."
      case .rpc(let message):
        return message
      }
    }
  }

  private(set) var connectionState: ConnectionState = .disconnected

  private var process: Process?
  private var inputHandle: FileHandle?
  private var outputHandle: FileHandle?
  private var nextRequestID: Int = 1

  func launchAndInitialize() async throws -> SessionInfo {
    if process == nil {
      try launchProcess()
    }

    connectionState = .launching

    let initializeParams = InitializeParams(
      clientInfo: ClientInfo(
        name: "cavell-macos",
        version: "0.1.0"
      )
    )

    let response: JSONRPCResponse<InitializeResult> = try await sendRequest(
      method: "initialize",
      params: initializeParams
    )

    if let error = response.error {
      throw RuntimeError.rpc(error.message)
    }

    guard let result = response.result else {
      throw RuntimeError.invalidResponse
    }

    connectionState = .ready

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

    if let error = response.error {
      throw RuntimeError.rpc(error.message)
    }

    guard let result = response.result else {
      throw RuntimeError.invalidResponse
    }

    return result.threads.map {
      RuntimeThreadSummary(id: $0.id, title: $0.title, status: $0.status)
    }
  }

  func openWorkspace(path: String) async throws -> RuntimeWorkspace {
    let response: JSONRPCResponse<WorkspaceOpenResult> = try await sendRequest(
      method: "workspace/open",
      params: WorkspaceOpenParams(path: path)
    )

    if let error = response.error {
      throw RuntimeError.rpc(error.message)
    }

    guard let result = response.result else {
      throw RuntimeError.invalidResponse
    }

    return RuntimeWorkspace(
      rootPath: result.workspace.rootPath,
      displayName: result.workspace.displayName,
      threadCount: result.threadCount
    )
  }

  func startThread(title: String) async throws -> ThreadSummary {
    let response: JSONRPCResponse<ThreadStartResult> = try await sendRequest(
      method: "thread/start",
      params: ThreadStartParams(title: title)
    )

    if let error = response.error {
      throw RuntimeError.rpc(error.message)
    }

    guard let result = response.result else {
      throw RuntimeError.invalidResponse
    }

    return ThreadSummary(
      id: result.thread.id,
      title: result.thread.title,
      preview: result.thread.status
    )
  }

  func startTurn(threadID: String, message: String) async throws -> RuntimeTurnResult {
    let response: JSONRPCResponse<TurnStartResult> = try await sendRequest(
      method: "turn/start",
      params: TurnStartParams(threadId: threadID, message: message)
    )

    if let error = response.error {
      throw RuntimeError.rpc(error.message)
    }

    guard let result = response.result else {
      throw RuntimeError.invalidResponse
    }

    return RuntimeTurnResult(
      turnID: result.turnId,
      threadID: result.threadId,
      items: result.items.map {
        RuntimeTimelineItemResult(kind: $0.kind, title: $0.title, content: $0.content)
      }
    )
  }

  func readThread(threadID: String) async throws -> RuntimeThreadState {
    let response: JSONRPCResponse<ThreadReadResult> = try await sendRequest(
      method: "thread/read",
      params: ThreadReadParams(threadId: threadID)
    )

    if let error = response.error {
      throw RuntimeError.rpc(error.message)
    }

    guard let result = response.result else {
      throw RuntimeError.invalidResponse
    }

    return RuntimeThreadState(
      id: result.thread.id,
      title: result.thread.title,
      status: result.thread.status,
      items: result.items.map {
        RuntimeTimelineItemResult(kind: $0.kind, title: $0.title, content: $0.content)
      }
    )
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

    try process.run()

    self.process = process
    inputHandle = stdinPipe.fileHandleForWriting
    outputHandle = stdoutPipe.fileHandleForReading
  }

  private func resolveRuntimeURL() throws -> URL {
    let environment = ProcessInfo.processInfo.environment

    if let customPath = environment["CAVELL_RUNTIME_PATH"], !customPath.isEmpty {
      return URL(fileURLWithPath: customPath)
    }

    if let bundledURL = Bundle.main.executableURL?
      .deletingLastPathComponent()
      .appendingPathComponent("cavell-runtime-bin"),
      FileManager.default.fileExists(atPath: bundledURL.path)
    {
      return bundledURL
    }

    throw RuntimeError.runtimePathMissing
  }

  private func runtimeEnvironment() -> [String: String] {
    var environment = ProcessInfo.processInfo.environment
    environment["CAVELL_DATA_DIR"] = appSupportStorageDirectory().path
    return environment
  }

  private func appSupportStorageDirectory() -> URL {
    let baseDirectory =
      FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
      ?? URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)

    return baseDirectory
      .appendingPathComponent("Cavell", isDirectory: true)
      .appendingPathComponent("storage", isDirectory: true)
  }

  private func sendRequest<Params: Encodable, ResultType: Decodable>(
    method: String,
    params: Params
  ) async throws -> JSONRPCResponse<ResultType> {
    guard let inputHandle, let outputHandle else {
      throw RuntimeError.runtimePipeUnavailable
    }

    let request = JSONRPCRequest(
      id: nextRequestID,
      method: method,
      params: params
    )
    nextRequestID += 1

    let encoder = JSONEncoder()
    let data = try encoder.encode(request) + Data([0x0A])
    try inputHandle.write(contentsOf: data)

    let line = try await Self.readLineAsync(from: outputHandle)

    let decoder = JSONDecoder()
    return try decoder.decode(JSONRPCResponse<ResultType>.self, from: Data(line.utf8))
  }

  private static func readLineAsync(from handle: FileHandle) async throws -> String {
    try await withCheckedThrowingContinuation { continuation in
      DispatchQueue.global(qos: .userInitiated).async {
        do {
          continuation.resume(returning: try readLine(from: handle))
        } catch {
          continuation.resume(throwing: error)
        }
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
      throw RuntimeError.invalidResponse
    }

    return String(decoding: data, as: UTF8.self)
  }
}
