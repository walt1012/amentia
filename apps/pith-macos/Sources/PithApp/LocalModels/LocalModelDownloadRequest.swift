import Foundation

enum LocalModelDownloadRequestMode {
  case start(downloadURL: URL)
  case blocked(detail: String)
}

struct LocalModelDownloadRequestPlan {
  let mode: LocalModelDownloadRequestMode

  var canStart: Bool {
    switch mode {
    case .start:
      return true
    case .blocked:
      return false
    }
  }

  var downloadURL: URL? {
    switch mode {
    case .start(let downloadURL):
      return downloadURL
    case .blocked:
      return nil
    }
  }

  var blockedDetail: String? {
    switch mode {
    case .start:
      return nil
    case .blocked(let detail):
      return detail
    }
  }
}

enum LocalModelDownloadRequestPlanner {
  static func plan(
    model: LocalModelSummary,
    isDownloadRunning: Bool,
    pausedModelID: String?,
    hasResumeData: Bool,
    resumeBytesReceived: Int64?
  ) -> LocalModelDownloadRequestPlan {
    if isDownloadRunning {
      return .blocked("Finish, pause, or cancel the current model download before starting another.")
    }

    if model.downloaded {
      return .blocked("\(model.displayName) is already downloaded.")
    }

    let isResumingSelectedModel: Bool
    if let pausedModelID {
      guard pausedModelID == model.id else {
        return .blocked("Continue or cancel the paused model download before starting another model.")
      }

      guard hasResumeData else {
        return .blocked("Cancel the paused model download before trying again.")
      }
      isResumingSelectedModel = true
    } else {
      isResumingSelectedModel = false
    }

    guard let downloadURL = URL(string: model.downloadURL) else {
      return .blocked("The selected local model has an invalid download URL.")
    }
    guard downloadURL.scheme == "https" else {
      return .blocked("The selected local model must be downloaded over HTTPS.")
    }

    let resumedBytes = isResumingSelectedModel ? resumeBytesReceived : nil
    if let blockedDetail = storageCapacityBlockedDetail(for: model, resumeBytesReceived: resumedBytes) {
      return .blocked(blockedDetail)
    }

    return .start(downloadURL: downloadURL)
  }

  private static func storageCapacityBlockedDetail(
    for model: LocalModelSummary,
    resumeBytesReceived: Int64?
  ) -> String? {
    guard let availableBytes = availableStorageBytes(for: model.installPath) else {
      return nil
    }

    let remainingBytes = storageBytesNeeded(for: model, resumeBytesReceived: resumeBytesReceived)
    let minimumBufferBytes: Int64
    if resumeBytesReceived == nil {
      minimumBufferBytes = 64 * 1024 * 1024
    } else {
      minimumBufferBytes = 16 * 1024 * 1024
    }
    let requiredBytes = remainingBytes + max(remainingBytes / 5, minimumBufferBytes)
    guard availableBytes < requiredBytes else {
      return nil
    }

    let operation = resumeBytesReceived == nil ? "downloading" : "continuing"
    return """
      Free at least \(formattedByteCount(requiredBytes)) on the local model volume before \(operation) \(model.displayName). \
      Available: \(formattedByteCount(availableBytes)).
      """
  }

  private static func storageBytesNeeded(
    for model: LocalModelSummary,
    resumeBytesReceived: Int64?
  ) -> Int64 {
    guard let resumeBytesReceived else {
      return model.sizeBytes
    }

    let boundedReceivedBytes = min(max(resumeBytesReceived, 0), model.sizeBytes)
    return max(model.sizeBytes - boundedReceivedBytes, 0)
  }

  private static func availableStorageBytes(for path: String) -> Int64? {
    let targetURL = URL(fileURLWithPath: path)
    guard let volumeURL = existingAncestor(for: targetURL) else {
      return nil
    }

    guard let attributes = try? FileManager.default.attributesOfFileSystem(
      forPath: volumeURL.path
    ),
          let freeSize = attributes[.systemFreeSize] as? NSNumber
    else {
      return nil
    }

    return freeSize.int64Value
  }

  private static func existingAncestor(for url: URL) -> URL? {
    let manager = FileManager.default
    var candidate = url.deletingLastPathComponent()

    while !manager.fileExists(atPath: candidate.path) {
      let parent = candidate.deletingLastPathComponent()
      guard parent.path != candidate.path else {
        return nil
      }
      candidate = parent
    }

    return candidate
  }

  private static func formattedByteCount(_ byteCount: Int64) -> String {
    let formatter = ByteCountFormatter()
    formatter.countStyle = .file
    return formatter.string(fromByteCount: byteCount)
  }
}

private extension LocalModelDownloadRequestPlan {
  static func blocked(_ detail: String) -> LocalModelDownloadRequestPlan {
    LocalModelDownloadRequestPlan(mode: .blocked(detail: detail))
  }

  static func start(downloadURL: URL) -> LocalModelDownloadRequestPlan {
    LocalModelDownloadRequestPlan(mode: .start(downloadURL: downloadURL))
  }
}
