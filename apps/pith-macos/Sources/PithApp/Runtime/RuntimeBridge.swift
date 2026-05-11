import Foundation

final class RuntimeBridge {
  var connectionState: ConnectionState {
    currentConnectionState()
  }
  var onThreadUpdated: ThreadUpdatedHandler?
  var onConnectionStateChanged: ConnectionStateHandler?

  private let connectionStateQueue = DispatchQueue(label: "pith.runtime.bridge.connection-state")
  private var connectionStateValue: ConnectionState = .disconnected
  private let processStateQueue = DispatchQueue(label: "pith.runtime.bridge.process-state")
  private var processSession: RuntimeBridgeProcessSession?
  private let messageDispatcher = RuntimeBridgeMessageDispatcher()
  private let pendingResponses = RuntimeBridgePendingResponses()
  private let requestWriter = RuntimeBridgeRequestWriter()

  func launchAndInitialize(launchDetail: String = "Launching local runtime") async throws -> SessionInfo {
    if currentProcessSession()?.isRunning != true {
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

    let result: InitializeResult
    do {
      let response: JSONRPCResponse<InitializeResult> = try await sendRequest(
        method: "initialize",
        params: initializeParams
      )
      result = try responseResult(from: response)
    } catch {
      guard connectionState != .failed else {
        throw error
      }

      let detail = "Runtime initialization failed: \(error.localizedDescription)"
      stopRuntimeAfterRequestBoundary(detail: detail)
      throw RuntimeError.rpc(detail)
    }

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
    let session = try RuntimeBridgeProcessSession(
      executableURL: executableURL,
      environment: runtimeEnvironment()
    )
    storeProcessSession(session)
    session.startObserving(
      onLine: { [weak self] processIdentifier, data in
        self?.handleIncomingMessage(
          processIdentifier: processIdentifier,
          data: data
        )
      },
      onReadError: { [weak self] processIdentifier, error in
        self?.handleProcessReadError(
          processIdentifier: processIdentifier,
          error: error
        )
      },
      onTermination: { [weak self] processIdentifier, detail in
        self?.handleProcessTermination(processIdentifier: processIdentifier, detail: detail)
      }
    )
    guard isCurrentProcessSession(session.identifier), session.isRunning else {
      detachProcessSession(matching: session.identifier)?.stop()
      throw RuntimeError.rpc("Runtime exited before initialization.")
    }
  }

  private func handleIncomingMessage(processIdentifier: ObjectIdentifier, data: Data) {
    guard isCurrentProcessSession(processIdentifier) else {
      return
    }

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

  private func handleProcessReadError(processIdentifier: ObjectIdentifier, error: Error) {
    guard let session = detachProcessSession(matching: processIdentifier) else {
      return
    }

    failPendingResponses(with: error)
    session.stop()
    updateConnectionState(.failed, detail: "Runtime disconnected.")
  }

  private func handleProcessTermination(processIdentifier: ObjectIdentifier, detail: String) {
    guard let session = detachProcessSession(matching: processIdentifier) else {
      return
    }

    failPendingResponses(with: RuntimeError.rpc(detail))
    session.stop()
    updateConnectionState(.failed, detail: detail)
  }

  private func resetProcessState() {
    let session = detachProcessSession()
    session?.stop()
  }

  private func currentProcessSession() -> RuntimeBridgeProcessSession? {
    processStateQueue.sync {
      processSession
    }
  }

  private func storeProcessSession(_ session: RuntimeBridgeProcessSession) {
    processStateQueue.sync {
      processSession = session
    }
  }

  private func isCurrentProcessSession(_ processIdentifier: ObjectIdentifier) -> Bool {
    processStateQueue.sync {
      processSession?.identifier == processIdentifier
    }
  }

  private func detachProcessSession(
    matching processIdentifier: ObjectIdentifier? = nil
  ) -> RuntimeBridgeProcessSession? {
    processStateQueue.sync {
      guard let session = processSession else {
        return nil
      }
      if let processIdentifier, session.identifier != processIdentifier {
        return nil
      }

      processSession = nil
      return session
    }
  }

  private func updateConnectionState(_ state: ConnectionState, detail: String) {
    connectionStateQueue.sync {
      connectionStateValue = state
    }
    onConnectionStateChanged?(state, detail)
  }

  private func currentConnectionState() -> ConnectionState {
    connectionStateQueue.sync {
      connectionStateValue
    }
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
    guard let inputHandle = currentProcessSession()?.inputHandle else {
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
            try requestWriter.write(payload, to: inputHandle)
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
