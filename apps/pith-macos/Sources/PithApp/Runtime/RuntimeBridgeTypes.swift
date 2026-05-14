import Foundation

extension RuntimeBridge {
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

  enum RuntimeError: LocalizedError {
    case runtimePathMissing
    case runtimePipeUnavailable
    case invalidResponse
    case requestTimedOut(method: String, seconds: Int)
    case rpc(String)
    case rpcWithRepair(message: String, repairHint: String)
    case rpcWithRecovery(message: String, repairHint: String?, attributes: [String: String])

    var errorDescription: String? {
      switch self {
      case .runtimePathMissing:
        return
          "The runtime binary could not be found. " +
          "Set PITH_RUNTIME_PATH to the built runtime executable."
      case .runtimePipeUnavailable:
        return "The runtime process pipes are not available."
      case .invalidResponse:
        return "The runtime returned an invalid response."
      case .requestTimedOut(let method, let seconds):
        return
          "Runtime request \(method) timed out after \(seconds) seconds. " +
          "The local runtime was stopped so it can recover cleanly."
      case .rpc(let message):
        return message
      case .rpcWithRepair(let message, let repairHint):
        return "\(message)\n\nRepair Hint: \(repairHint)"
      case .rpcWithRecovery(let message, let repairHint, _):
        guard let repairHint, !repairHint.isEmpty else {
          return message
        }
        return "\(message)\n\nRepair Hint: \(repairHint)"
      }
    }

    var recoveryAttributes: [String: String] {
      switch self {
      case .rpcWithRecovery(_, _, let attributes):
        return attributes
      default:
        return [:]
      }
    }
  }

  typealias ThreadUpdatedHandler = @Sendable (RuntimeThreadState) -> Void
  typealias ConnectionStateHandler = @Sendable (ConnectionState, String) -> Void
}
