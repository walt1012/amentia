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

  func launchAndInitialize(launchDetail: String = "Starting local service") async throws -> SessionInfo {
    if currentProcessSession()?.isRunning != true {
      resetProcessState()
      let environment = await runtimeEnvironment()
      try launchProcess(environment: environment)
    }

    updateConnectionState(.launching, detail: launchDetail)

    let initializeParams = InitializeParams(
      clientInfo: ClientInfo(
        name: "pith-macos",
        version: appBundleVersion()
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

      let detail = stopRuntimeAfterRequestBoundary(
        detail: "Local service initialization failed: \(error.localizedDescription)"
      )
      throw RuntimeError.rpc(detail)
    }

    updateConnectionState(.ready, detail: "\(result.serverInfo.name) \(result.serverInfo.version)")

    return SessionInfo(
      serverName: result.serverInfo.name,
      serverVersion: result.serverInfo.version
    )
  }

  func stopRuntime(detail: String = "Local service stopped.") {
    failPendingResponses(with: RuntimeError.rpc(detail))
    resetProcessState()
    updateConnectionState(.disconnected, detail: detail)
  }

  private func appBundleVersion() -> String {
    let fallbackVersion = "0.1.0"
    guard let version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String else {
      return fallbackVersion
    }

    let trimmedVersion = version.trimmingCharacters(in: .whitespacesAndNewlines)
    return trimmedVersion.isEmpty ? fallbackVersion : trimmedVersion
  }

  private func launchProcess(environment: [String: String]) throws {
    let executableURL = try resolveRuntimeURL()
    let session = try RuntimeBridgeProcessSession(
      executableURL: executableURL,
      environment: environment
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
      throw RuntimeError.rpc("Local service exited before initialization.")
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
    if RuntimeBridgeRequestPolicy.shouldStopRuntimeAfterTimedOutRequest(method: method) {
      stopRuntimeAfterRequestTimeout(method: method, seconds: seconds)
    }
  }

  private func handleRequestCancellation(requestID: Int, method: String) {
    guard let continuation = takePendingResponse(requestID: requestID) else {
      return
    }

    let detail = "Local service request \(method) was cancelled."
    continuation.resume(throwing: RuntimeError.rpc(detail))
    if RuntimeBridgeRequestPolicy.shouldStopRuntimeAfterCancelledRequest(method: method) {
      stopRuntimeAfterRequestCancellation(method: method)
    }
  }

  private func stopRuntimeAfterRequestCancellation(method: String) {
    let detail =
      "Local service request \(method) was cancelled. " +
      "Restart the local service to continue."
    stopRuntimeAfterRequestBoundary(detail: detail)
  }

  private func stopRuntimeAfterRequestTimeout(method: String, seconds: Int) {
    let detail =
      "Local service request \(method) timed out after \(seconds) seconds. " +
      "Restart the local service to continue."
    stopRuntimeAfterRequestBoundary(detail: detail)
  }

  @discardableResult
  private func stopRuntimeAfterRequestBoundary(detail: String) -> String {
    let detail = runtimeFailureDetail(detail)
    failPendingResponses(with: RuntimeError.rpc(detail))
    resetProcessState()
    updateConnectionState(.failed, detail: detail)
    return detail
  }

  private func failPendingResponses(with error: Error) {
    pendingResponses.failAll(with: error)
  }

  private func handleProcessReadError(processIdentifier: ObjectIdentifier, error: Error) {
    guard let session = detachProcessSession(matching: processIdentifier) else {
      return
    }

    let detail = runtimeFailureDetail("Local service disconnected.", session: session)
    failPendingResponses(with: RuntimeError.rpc(detail))
    session.stop()
    updateConnectionState(.failed, detail: detail)
  }

  private func handleProcessTermination(processIdentifier: ObjectIdentifier, detail: String) {
    guard let session = detachProcessSession(matching: processIdentifier) else {
      return
    }

    let detail = runtimeFailureDetail(detail, session: session)
    failPendingResponses(with: RuntimeError.rpc(detail))
    session.stop()
    updateConnectionState(.failed, detail: detail)
  }

  private func runtimeFailureDetail(
    _ detail: String,
    session: RuntimeBridgeProcessSession? = nil
  ) -> String {
    guard let summary = (session ?? currentProcessSession())?.recentErrorSummary else {
      return detail
    }

    return "\(detail) Local service log: \(summary)"
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

  private func runtimeEnvironment() async -> [String: String] {
    await RuntimeBridgeLocalEnvironment.runtimeEnvironment()
  }

  func sendRequest<Params: Encodable, ResultType: Decodable>(
    method: String,
    params: Params
  ) async throws -> JSONRPCResponse<ResultType> {
    guard let inputHandle = currentProcessSession()?.inputHandle else {
      throw RuntimeError.runtimePipeUnavailable
    }

    let requestID = pendingResponses.reserveRequestID()
    let request = JSONRPCRequest(
      id: requestID,
      method: method,
      params: params
    )
    let encoder = JSONEncoder()
    let payload = try encoder.encode(request) + Data([0x0A])
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
            try requestWriter.write(payload, to: inputHandle)
          } catch {
            stopRuntimeAfterRequestWriteFailure(method: method, error: error)
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

  private func stopRuntimeAfterRequestWriteFailure(method: String, error: Error) {
    let detail =
      "Local service request \(method) could not be written: \(error.localizedDescription). " +
      "Restart the local service to continue."
    stopRuntimeAfterRequestBoundary(detail: detail)
  }

}
