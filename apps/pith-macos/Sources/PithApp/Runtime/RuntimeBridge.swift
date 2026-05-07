import Foundation

final class RuntimeBridge {
  private(set) var connectionState: ConnectionState = .disconnected
  var onThreadUpdated: ThreadUpdatedHandler?
  var onConnectionStateChanged: ConnectionStateHandler?

  private var process: Process?
  private var inputHandle: FileHandle?
  private var outputHandle: FileHandle?
  private var nextRequestID: Int = 1
  private let stateQueue = DispatchQueue(label: "pith.runtime.bridge.state")
  private var pendingResponses: [Int: CheckedContinuation<Data, Error>] = [:]
  private var readerTask: Task<Void, Never>?
  private var errorReaderTask: Task<Void, Never>?

  func launchAndInitialize(launchDetail: String = "Launching local runtime") async throws -> SessionInfo {
    if process == nil || process?.isRunning != true {
      resetProcessState()
      try launchProcess()
    }

    updateConnectionState(.launching, detail: launchDetail)

    let initializeParams = InitializeParams(
      clientInfo: ClientInfo(
        name: "pith-macos",
        version: "0.1.0"
      )
    )

    let response: JSONRPCResponse<InitializeResult> = try await sendRequest(
      method: "initialize",
      params: initializeParams
    )
    let result = try responseResult(from: response)

    updateConnectionState(.ready, detail: "\(result.serverInfo.name) \(result.serverInfo.version)")

    return SessionInfo(
      serverName: result.serverInfo.name,
      serverVersion: result.serverInfo.version
    )
  }

  func localPluginInstallRootPath() -> String {
    RuntimeBridgeLocalEnvironment.localPluginInstallRootPath()
  }

  func localModelStorageRootPath() -> String {
    RuntimeBridgeLocalEnvironment.localModelStorageRootPath()
  }

  func activeLocalModelPath() -> String? {
    RuntimeBridgeLocalEnvironment.activeLocalModelPath()
  }

  func configureActiveLocalModel(manifestPath: String, modelPath: String) {
    RuntimeBridgeLocalEnvironment.configureActiveLocalModel(
      manifestPath: manifestPath,
      modelPath: modelPath
    )
  }

  func clearActiveLocalModel() {
    RuntimeBridgeLocalEnvironment.clearActiveLocalModel()
  }

  func stopRuntime(detail: String = "Runtime stopped.") {
    failPendingResponses(with: RuntimeError.rpc(detail))
    if let process, process.isRunning {
      process.terminationHandler = nil
      process.terminate()
    }
    resetProcessState()
    updateConnectionState(.disconnected, detail: detail)
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
    let processIdentifier = ObjectIdentifier(process)
    process.terminationHandler = { [weak self] process in
      let detail = "Runtime exited with status \(process.terminationStatus)."
      self?.handleProcessTermination(processIdentifier: processIdentifier, detail: detail)
    }

    try process.run()

    self.process = process
    inputHandle = stdinPipe.fileHandleForWriting
    outputHandle = stdoutPipe.fileHandleForReading
    startReaderLoop(with: stdoutPipe.fileHandleForReading, processIdentifier: processIdentifier)
    startErrorReaderLoop(with: stderrPipe.fileHandleForReading)
  }

  private func startReaderLoop(with handle: FileHandle, processIdentifier: ObjectIdentifier) {
    readerTask?.cancel()
    readerTask = Task.detached(priority: .userInitiated) { [weak self] in
      guard let self else {
        return
      }

      while !Task.isCancelled {
        let line: String
        do {
          line = try RuntimeBridgeLineReader.readLine(from: handle)
        } catch {
          self.failPendingResponses(with: error)
          self.handleProcessTermination(
            processIdentifier: processIdentifier,
            detail: "Runtime disconnected."
          )
          return
        }

        let data = Data(line.utf8)
        self.handleIncomingMessage(data)
      }
    }
  }

  private func handleIncomingMessage(_ data: Data) {
    let decoder = JSONDecoder()

    if let response = try? decoder.decode(JSONRPCAnyResponse.self, from: data),
       let responseID = response.id
    {
      let continuation = takePendingResponse(requestID: responseID)
      continuation?.resume(returning: data)
      return
    }

    guard let envelope = try? decoder.decode(JSONRPCNotificationEnvelope.self, from: data) else {
      return
    }

    if envelope.method == "thread/updated",
       let notification = try? decoder.decode(
         JSONRPCNotification<ThreadUpdatedNotificationParams>.self,
         from: data
       )
    {
      let state = RuntimeBridgePayloadMapper.threadState(
        id: notification.params.thread.id,
        title: notification.params.thread.title,
        status: notification.params.thread.status,
        items: notification.params.items,
        pendingApprovals: notification.params.pendingApprovals,
        activeTurnID: notification.params.activeTurnId
      )
      onThreadUpdated?(state)
    }
  }

  private func startErrorReaderLoop(with handle: FileHandle) {
    errorReaderTask?.cancel()
    errorReaderTask = Task.detached(priority: .utility) {
      while !Task.isCancelled {
        do {
          let chunk = try handle.read(upToCount: 4096) ?? Data()
          if chunk.isEmpty {
            return
          }

          #if DEBUG
            if let rawMessage = String(data: chunk, encoding: .utf8) {
              let message = rawMessage.trimmingCharacters(in: .whitespacesAndNewlines)
              guard !message.isEmpty else {
                continue
              }
              print("[pith-runtime stderr] \(message)")
            }
          #endif
        } catch {
          return
        }
      }
    }
  }

  private func takePendingResponse(requestID: Int) -> CheckedContinuation<Data, Error>? {
    stateQueue.sync {
      pendingResponses.removeValue(forKey: requestID)
    }
  }

  private func handleRequestTimeout(requestID: Int, method: String, timeoutNanoseconds: UInt64) {
    guard let continuation = takePendingResponse(requestID: requestID) else {
      return
    }

    let seconds = RuntimeBridgeRequestPolicy.timeoutSeconds(from: timeoutNanoseconds)
    let error = RuntimeError.requestTimedOut(method: method, seconds: seconds)
    continuation.resume(throwing: error)
    stopRuntimeAfterRequestTimeout(method: method, seconds: seconds)
  }

  private func handleRequestCancellation(requestID: Int, method: String) {
    guard let continuation = takePendingResponse(requestID: requestID) else {
      return
    }

    let detail = "Runtime request \(method) was cancelled."
    continuation.resume(throwing: RuntimeError.rpc(detail))
    if RuntimeBridgeRequestPolicy.shouldStopRuntimeAfterCancelledRequest(method: method) {
      stopRuntimeAfterRequestCancellation(method: method)
    }
  }

  private func stopRuntimeAfterRequestCancellation(method: String) {
    let detail =
      "Runtime request \(method) was cancelled. " +
      "Relaunch the local runtime to continue."
    stopRuntimeAfterRequestBoundary(detail: detail)
  }

  private func stopRuntimeAfterRequestTimeout(method: String, seconds: Int) {
    let detail =
      "Runtime request \(method) timed out after \(seconds) seconds. " +
      "Relaunch the local runtime to continue."
    stopRuntimeAfterRequestBoundary(detail: detail)
  }

  private func stopRuntimeAfterRequestBoundary(detail: String) {
    failPendingResponses(with: RuntimeError.rpc(detail))
    if let process, process.isRunning {
      process.terminationHandler = nil
      process.terminate()
    }
    resetProcessState()
    updateConnectionState(.failed, detail: detail)
  }

  private func failPendingResponses(with error: Error) {
    let continuations = stateQueue.sync {
      let continuations = Array(pendingResponses.values)
      pendingResponses.removeAll()
      return continuations
    }
    for continuation in continuations {
      continuation.resume(throwing: error)
    }
  }

  private func handleProcessTermination(processIdentifier: ObjectIdentifier, detail: String) {
    guard let process, ObjectIdentifier(process) == processIdentifier else {
      return
    }

    failPendingResponses(with: RuntimeError.rpc(detail))
    resetProcessState()
    updateConnectionState(.failed, detail: detail)
  }

  private func resetProcessState() {
    readerTask?.cancel()
    readerTask = nil
    errorReaderTask?.cancel()
    errorReaderTask = nil

    if let process, process.isRunning {
      process.terminationHandler = nil
    }

    process = nil
    inputHandle = nil
    outputHandle = nil
  }

  private func updateConnectionState(_ state: ConnectionState, detail: String) {
    connectionState = state
    onConnectionStateChanged?(state, detail)
  }

  private func resolveRuntimeURL() throws -> URL {
    let environment = ProcessInfo.processInfo.environment

    if let customPath = environment["PITH_RUNTIME_PATH"], !customPath.isEmpty {
      return URL(fileURLWithPath: customPath)
    }

    if let bundledURL = Bundle.main.executableURL?
      .deletingLastPathComponent()
      .appendingPathComponent("pith-runtime-bin"),
      FileManager.default.fileExists(atPath: bundledURL.path)
    {
      return bundledURL
    }

    throw RuntimeError.runtimePathMissing
  }

  private func runtimeEnvironment() -> [String: String] {
    RuntimeBridgeLocalEnvironment.runtimeEnvironment()
  }

  func sendRequest<Params: Encodable, ResultType: Decodable>(
    method: String,
    params: Params
  ) async throws -> JSONRPCResponse<ResultType> {
    guard let inputHandle else {
      throw RuntimeError.runtimePipeUnavailable
    }

    let requestID = stateQueue.sync { () -> Int in
      let id = nextRequestID
      nextRequestID += 1
      return id
    }
    let timeoutNanoseconds = RuntimeBridgeRequestPolicy.timeoutNanoseconds(for: method)
    let timeoutTask = Task { [weak self] in
      do {
        try await Task.sleep(nanoseconds: timeoutNanoseconds)
      } catch {
        return
      }
      guard !Task.isCancelled else {
        return
      }
      self?.handleRequestTimeout(
        requestID: requestID,
        method: method,
        timeoutNanoseconds: timeoutNanoseconds
      )
    }

    let data: Data
    do {
      data = try await withTaskCancellationHandler(operation: {
        try await withCheckedThrowingContinuation { continuation in
          stateQueue.sync {
            self.pendingResponses[requestID] = continuation
          }

          do {
            let request = JSONRPCRequest(
              id: requestID,
              method: method,
              params: params
            )
            let encoder = JSONEncoder()
            let payload = try encoder.encode(request) + Data([0x0A])
            try inputHandle.write(contentsOf: payload)
          } catch {
            let pending = takePendingResponse(requestID: requestID)
            pending?.resume(throwing: error)
          }
        }
      }, onCancel: {
        timeoutTask.cancel()
        handleRequestCancellation(requestID: requestID, method: method)
      })
    } catch {
      timeoutTask.cancel()
      throw error
    }
    timeoutTask.cancel()

    let decoder = JSONDecoder()
    return try decoder.decode(JSONRPCResponse<ResultType>.self, from: data)
  }

  func responseResult<ResultType: Decodable>(
    from response: JSONRPCResponse<ResultType>
  ) throws -> ResultType {
    if let error = response.error {
      throw RuntimeError.rpc(error.message)
    }

    guard let result = response.result else {
      throw RuntimeError.invalidResponse
    }

    return result
  }

}
