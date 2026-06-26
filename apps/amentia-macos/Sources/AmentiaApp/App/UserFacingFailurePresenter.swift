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

  static func fileRevealFailureDetail() -> String {
    "Amentia could not open that folder. Check Finder permissions, then try again."
  }

  static func projectSearchFailureStatus() -> String {
    "Project search needs attention. Check Amentia status, then try again."
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

  static func workspaceRestoreFailureBody() -> String {
    "Could not restore the last project. Open a project folder to continue."
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

  static func pluginActionFailureBody(isBlocked: Bool) -> String {
    isBlocked
      ? "Plugin action needs attention. Select this item for the repair hint."
      : "Plugin action failed. Select this item for details, then try again."
  }

  static func pluginInstallFailureBody(repairHint: String) -> String {
    let base = "Plugin install failed. Check the plugin source, then try again."
    let trimmedHint = repairHint.trimmingCharacters(in: .whitespacesAndNewlines)
    return trimmedHint.isEmpty ? base : "\(base)\n\nRepair Hint: \(trimmedHint)"
  }

  static func pluginPreviewFailureBody(repairHint: String) -> String {
    let base = "Plugin preview failed. Check the plugin source, then try again."
    let trimmedHint = repairHint.trimmingCharacters(in: .whitespacesAndNewlines)
    return trimmedHint.isEmpty ? base : "\(base)\n\nRepair Hint: \(trimmedHint)"
  }

  static func pluginLifecycleFailureBody(action: String) -> String {
    "Plugin \(action) failed. Check plugin setup, then try again."
  }

  static func pluginRefreshDiagnostic(label: String) -> String {
    switch label {
    case "catalog refresh", "catalog":
      return "Plugin catalog needs attention."
    case "capability registry", "command registry", "connector registry",
         "hook registry", "skill registry":
      return "Plugin capabilities need attention."
    default:
      return "Plugin setup needs attention."
    }
  }

  static func connectionAuthorizationFailureBody() -> String {
    "Connection authorization failed. Check the saved token or key, then try again."
  }

  static func connectionAuthorizationClearFailureBody() -> String {
    "Could not clear connection authorization. Restart Amentia, then try again."
  }

  static func technicalErrorAttributes(_ error: Error) -> [String: String] {
    [
      "technicalError": error.localizedDescription
    ]
  }
}
