import Foundation

enum LocalModelDownloadStartMode {
  case newDownload
  case resuming(resumeData: Data)
}

struct LocalModelDownloadStartPlan {
  let mode: LocalModelDownloadStartMode
  let progress: ModelDownloadProgress
  let runtimeDetail: String
  let timelineTitle: String
  let timelineBody: String
  let attributes: [String: String]

  var isResuming: Bool {
    switch mode {
    case .newDownload:
      return false
    case .resuming:
      return true
    }
  }

  var resumeData: Data? {
    switch mode {
    case .newDownload:
      return nil
    case .resuming(let data):
      return data
    }
  }
}

enum LocalModelDownloadStartPlanner {
  static func plan(
    model: LocalModelSummary,
    sourceURL: URL,
    pausedModelID: String?,
    resumeData: Data?,
    currentProgress: ModelDownloadProgress?
  ) -> LocalModelDownloadStartPlan {
    let now = Date()
    let mode: LocalModelDownloadStartMode
    let resumedBytes: Int64
    let isResuming: Bool

    if pausedModelID == model.id, let resumeData {
      mode = .resuming(resumeData: resumeData)
      resumedBytes = currentProgress?.modelID == model.id
        ? currentProgress?.bytesReceived ?? 0
        : 0
      isResuming = true
    } else {
      mode = .newDownload
      resumedBytes = 0
      isResuming = false
    }

    let verb = isResuming ? "Continuing" : "Downloading"
    let eventVerb = isResuming ? "continued" : "started"

    return LocalModelDownloadStartPlan(
      mode: mode,
      progress: ModelDownloadProgress(
        modelID: model.id,
        displayName: model.displayName,
        bytesReceived: resumedBytes,
        totalBytes: model.sizeBytes,
        startedAt: now,
        updatedAt: now,
        isResuming: isResuming
      ),
      runtimeDetail: "\(verb) \(model.displayName) (\(formattedByteCount(model.sizeBytes)))...",
      timelineTitle: isResuming ? "Local Model Download Continued" : "Local Model Download Started",
      timelineBody:
        "\(model.displayName) download \(eventVerb) from \(sourceURL.absoluteString).",
      attributes: [
        "downloadUrl": sourceURL.absoluteString,
        "result": isResuming ? "continued" : "started",
        "size": formattedByteCount(model.sizeBytes),
      ]
    )
  }

  private static func formattedByteCount(_ byteCount: Int64) -> String {
    let formatter = ByteCountFormatter()
    formatter.countStyle = .file
    return formatter.string(fromByteCount: byteCount)
  }
}
