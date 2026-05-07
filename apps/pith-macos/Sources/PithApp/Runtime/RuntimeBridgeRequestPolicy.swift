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
    method == "turn/start" || method == "plugin/commandRun"
  }
}
