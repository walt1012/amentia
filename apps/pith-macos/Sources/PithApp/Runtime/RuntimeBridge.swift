import Foundation

final class RuntimeBridge {
  private(set) var connectionState: ConnectionState = .disconnected
  var onThreadUpdated: ThreadUpdatedHandler?
  var onConnectionStateChanged: ConnectionStateHandler?

  private var processSession: RuntimeBridgeProcessSession?
  private let messageDispatcher = RuntimeBridgeMessageDispatcher()
  private let pendingResponses = RuntimeBridgePendingResponses()

  func launchAndInitialize(launchDetail: String = "Launching local runtime") async throws -> SessionInfo {
    if processSession?.isRunning != true {
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

  func stopRuntime(detail: String = "Runtime stopped.") {
    failPendingResponses(with: RuntimeError.rpc(detail))
    resetProcessState()
    updateConnectionState(.disconnected, detail: detail)
  }

  private func launchProcess() throws {
    let executableURL = try resolveRuntimeURL()
    processSession = try RuntimeBridgeProcessSession(
      executableURL: executableURL,
      environment: runtimeEnvironment(),
      onLine: { [weak self] data in
        self?.handleIncomingMessage(data)
      },
      onReadError: { [weak self] processIdentifier, error in
        self?.failPendingResponses(with: error)
        self?.handleProcessTermination(
          processIdentifier: processIdentifier,
          detail: "Runtime disconnected."
        )
      },
      onTermination: { [weak self] processIdentifier, detail in
        self?.handleProcessTermination(processIdentifier: processIdentifier, detail: detail)
      }
    )
  }

  private func handleIncomingMessage(_ data: Data) {
    switch messageDispatcher.decode(data) {
    case .response(let responseID, let data):
      let continuation = takePendingResponse(requestID: responseID)
      continuation?.resume(returning: data)
    case .threadUpdated(let state):
      onThreadUpdated?(state)
    case .ignored:
      return
    }
  }

  private func takePendingResponse(requestID: Int) -> CheckedContinuation<Data, Error>? {
    pendingResponses.take(requestID: requestID)
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
    resetProcessState()
    updateConnectionState(.failed, detail: detail)
  }

  private func failPendingResponses(with error: Error) {
    pendingResponses.failAll(with: error)
  }

  private func handleProcessTermination(processIdentifier: ObjectIdentifier, detail: String) {
    guard processSession?.identifier == processIdentifier else {
      return
    }

    failPendingResponses(with: RuntimeError.rpc(detail))
    resetProcessState()
    updateConnectionState(.failed, detail: detail)
  }

  private func resetProcessState() {
    processSession?.stop()
    processSession = nil
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
    guard let inputHandle = processSession?.inputHandle else {
      throw RuntimeError.runtimePipeUnavailable
    }

    let requestID = pendingResponses.reserveRequestID()
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
          pendingResponses.store(continuation, requestID: requestID)

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

}
