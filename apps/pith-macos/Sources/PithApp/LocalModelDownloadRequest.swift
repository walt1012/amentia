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
    hasResumeData: Bool
  ) -> LocalModelDownloadRequestPlan {
    if isDownloadRunning {
      return .blocked("Finish, pause, or cancel the current model download before starting another.")
    }

    if model.downloaded {
      return .blocked("\(model.displayName) is already downloaded.")
    }

    if let pausedModelID {
      guard pausedModelID == model.id else {
        return .blocked("Continue or cancel the paused model download before starting another model.")
      }

      guard hasResumeData else {
        return .blocked("Cancel the paused model download before trying again.")
      }
    }

    guard let downloadURL = URL(string: model.downloadURL) else {
      return .blocked("The selected local model has an invalid download URL.")
    }
    guard downloadURL.scheme == "https" else {
      return .blocked("The selected local model must be downloaded over HTTPS.")
    }

    if let blockedDetail = storageCapacityBlockedDetail(for: model) {
      return .blocked(blockedDetail)
    }

    return .start(downloadURL: downloadURL)
  }

  private static func storageCapacityBlockedDetail(for model: LocalModelSummary) -> String? {
    guard let availableBytes = availableStorageBytes(for: model.installPath) else {
      return nil
    }

    let requiredBytes = model.sizeBytes + max(model.sizeBytes / 5, 64 * 1024 * 1024)
    guard availableBytes < requiredBytes else {
      return nil
    }

    return """
      Free at least \(formattedByteCount(requiredBytes)) on the local model volume before downloading \(model.displayName). \
      Available: \(formattedByteCount(availableBytes)).
      """
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
