import Foundation

enum LocalModelDownloadInterruptionMode {
  case paused(resumeData: Data)
  case cancelled
  case failed
}

struct LocalModelDownloadInterruptionPlan {
  let mode: LocalModelDownloadInterruptionMode
  let runtimeDetail: String
  let timelineTitle: String
  let timelineBody: String
  let timelineKind: TimelineEntry.Kind
  let attributes: [String: String]
  let clearsPausedState: Bool
  let clearsProgress: Bool
  let removesPartialFile: Bool
}

enum LocalModelDownloadInterruptionPlanner {
  static func plan(model: LocalModelSummary, error: Error) -> LocalModelDownloadInterruptionPlan {
    if let paused = error as? ModelDownloadPaused {
      return LocalModelDownloadInterruptionPlan(
        mode: .paused(resumeData: paused.resumeData),
        runtimeDetail: "Paused \(model.displayName) download. Continue to resume from the saved partial state.",
        timelineTitle: "Local Model Download Paused",
        timelineBody:
          "\(model.displayName) download was paused and can continue from the saved local state.",
        timelineKind: .system,
        attributes: [
          "result": "paused"
        ],
        clearsPausedState: false,
        clearsProgress: false,
        removesPartialFile: false
      )
    }

    if isCancellation(error) {
      return cancellationPlan(model: model)
    }

    return LocalModelDownloadInterruptionPlan(
      mode: .failed,
      runtimeDetail: "Model download failed: \(error.localizedDescription)",
      timelineTitle: "Local Model Download Failed",
      timelineBody: "\(model.displayName) download failed: \(error.localizedDescription)",
      timelineKind: .warning,
      attributes: [
        "error": error.localizedDescription,
        "result": "failed",
      ],
      clearsPausedState: true,
      clearsProgress: true,
      removesPartialFile: false
    )
  }

  static func cancellationPlan(model: LocalModelSummary) -> LocalModelDownloadInterruptionPlan {
    LocalModelDownloadInterruptionPlan(
      mode: .cancelled,
      runtimeDetail: "Cancelled \(model.displayName) download and cleared partial state.",
      timelineTitle: "Local Model Download Cancelled",
      timelineBody: "\(model.displayName) download was cancelled and the partial file was cleared.",
      timelineKind: .system,
      attributes: [
        "result": "cancelled"
      ],
      clearsPausedState: true,
      clearsProgress: true,
      removesPartialFile: true
    )
  }

  private static func isCancellation(_ error: Error) -> Bool {
    if error is CancellationError {
      return true
    }

    return (error as? URLError)?.code == .cancelled
  }
}
