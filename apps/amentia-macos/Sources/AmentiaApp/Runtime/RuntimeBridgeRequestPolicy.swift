import Foundation

enum RuntimeBridgeRequestPolicy {
  private static let defaultRequestTimeoutNanoseconds: UInt64 = 30_000_000_000
  private static let turnRequestTimeoutNanoseconds: UInt64 = 210_000_000_000

  static func timeoutNanoseconds(for method: String) -> UInt64 {
    switch method {
    case "turn/start", "plugin/commandRun":
      return turnRequestTimeoutNanoseconds
    default:
      return defaultRequestTimeoutNanoseconds
    }
  }

  static func timeoutSeconds(from timeoutNanoseconds: UInt64) -> Int {
    max(Int(timeoutNanoseconds / 1_000_000_000), 1)
  }

  static func shouldStopRuntimeAfterCancelledRequest(method: String) -> Bool {
    switch method {
    case "turn/start",
         "plugin/commandRun",
         "plugin/inspect",
         "plugin/install",
         "plugin/setEnabled",
         "plugin/remove",
         "plugin/connectorAuthorize",
         "plugin/connectorClearCredential":
      return true
    default:
      return false
    }
  }

  static func shouldStopRuntimeAfterTimedOutRequest(method: String) -> Bool {
    switch method {
    case "workspace/search", "workspace/searchCancelRunning", "plugin/refresh":
      return false
    default:
      return true
    }
  }

  static func userFacingRequestName(for method: String) -> String {
    switch method {
    case "turn/start":
      return "current request"
    case "model/probe":
      return "model startup"
    case "workspace/search":
      return "project search"
    case "plugin/commandRun":
      return "action"
    case "plugin/install":
      return "plugin install"
    case "plugin/remove":
      return "plugin removal"
    case "plugin/refresh":
      return "plugin refresh"
    case "plugin/connectorAuthorize":
      return "connection authorization"
    case "plugin/connectorClearCredential":
      return "connection reset"
    default:
      return "current request"
    }
  }
}
