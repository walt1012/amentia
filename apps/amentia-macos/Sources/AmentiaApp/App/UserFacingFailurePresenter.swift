import Foundation

enum UserFacingFailurePresenter {
  static func appSupportDirectoryFailureDetail() -> String {
    "Amentia could not prepare its local data folder. Restart your Mac or choose a normal user account, then try again."
  }

  static func runtimeInitializationFailureDetail() -> String {
    "Amentia could not finish starting. Restart Amentia or install a fresh copy, then try again."
  }

  static func runtimeLaunchFailureDetail(error: Error) -> String {
    if let runtimeError = error as? RuntimeBridge.RuntimeError {
      switch runtimeError {
      case .runtimePathMissing, .requestTimedOut:
        return runtimeError.localizedDescription
      case .runtimePipeUnavailable:
        return "Amentia is not available. Restart Amentia to continue."
      case .invalidResponse, .rpc, .rpcWithRepair, .rpcWithRecovery:
        break
      }
    }

    return runtimeInitializationFailureDetail()
  }

  static func runtimeStoppedUnexpectedlyDetail() -> String {
    "Amentia stopped unexpectedly. Restart Amentia to continue."
  }

  static func requestWriteFailureDetail(method: String) -> String {
    let requestName = RuntimeBridgeRequestPolicy.userFacingRequestName(for: method)
    return "The \(requestName) could not be sent to Amentia. Restart Amentia to continue."
  }

  static func localDataDeletionFailureDetail() -> String {
    "Could not delete all local Amentia data. Close Amentia, reopen it, and try Delete All Local Data again."
  }

  static func threadCreationFailureBody() -> String {
    "Could not create a new session. Restart Amentia, then try again."
  }

  static func threadLoadFailureBody() -> String {
    "Could not load sessions. Restart Amentia, then try again."
  }

  static func workspaceOpenFailureBody() -> String {
    "Could not open that project. Choose a folder you can access, then try again."
  }

  static func approvalResponseFailureBody() -> String {
    "Could not send your approval choice. Restart Amentia, then try again."
  }

  static func approvalResponseFailureDetail() -> String {
    "Approval response could not be sent. Restart Amentia, then try again."
  }

  static func requestCancelFailureBody() -> String {
    "Could not cancel the request. If it keeps running, restart Amentia."
  }

  static func requestCancelFailureDetail() -> String {
    "Cancel did not finish. If the request keeps running, restart Amentia."
  }

  static func memoryNoteFailureBody() -> String {
    "Could not save that memory note. Try again after the current request finishes."
  }

  static func technicalErrorAttributes(_ error: Error) -> [String: String] {
    [
      "technicalError": error.localizedDescription
    ]
  }
}
