import Foundation

struct LocalModelStatusSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let modelHealth: ModelHealthSummary?
  let modelDownloadID: String?
  let pausedModelDownloadID: String?
  let modelDownloadProgress: ModelDownloadProgress?
  let selectedSetupModelID: String
  let selectedSetupModel: LocalModelSummary?
}

enum LocalModelStatusPresenter {
  static func displayName(_ snapshot: LocalModelStatusSnapshot) -> String {
    snapshot.modelHealth?.displayName ?? "Local Model Not Loaded"
  }

  static func statusSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "Launch the runtime to inspect local model health."
    }

    return "\(modelHealth.backend) | \(modelHealth.status)"
  }

  static func showsActivity(_ snapshot: LocalModelStatusSnapshot) -> Bool {
    snapshot.runtimeState == .launching || snapshot.modelDownloadID != nil
  }

  static func detailSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "Pith will use the built-in local model path after the runtime connects."
    }

    return modelHealth.detail
  }

  static func sourceSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "Source: unavailable"
    }

    let source = "Source: \(modelHealth.source)"
    if let manifestPath = modelHealth.manifestPath {
      return "\(source)\nManifest: \(manifestPath)"
    }

    return source
  }

  static func metricsSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "Metrics: unavailable"
    }

    let contextSize = modelHealth.metrics["contextSize"] ?? "unknown"
    let maxOutputTokens = modelHealth.metrics["maxOutputTokens"] ?? "unknown"
    let backend = modelHealth.metrics["backend"] ?? modelHealth.backend
    return "Context: \(contextSize) | Max Output: \(maxOutputTokens) | Backend: \(backend)"
  }

  static func readinessSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "Readiness: unavailable"
    }

    let readiness = modelHealth.metrics["readiness"] ?? "unknown"
    let packReady = modelHealth.metrics["packReady"] ?? "false"
    return "Readiness: \(readiness) | Pack Ready: \(packReady)"
  }

  static func installHintSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "Install hint: launch the runtime to inspect local model setup."
    }

    return modelHealth.metrics["installHint"] ?? "Install hint unavailable."
  }

  static func suggestedPathSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "Suggested install layout unavailable."
    }

    let manifestPath = modelHealth.metrics["suggestedManifestPath"] ?? "manifest path unavailable"
    let modelPath = modelHealth.metrics["suggestedModelPath"] ?? "model path unavailable"
    let binaryPath = modelHealth.metrics["suggestedBinaryPath"] ?? "binary path unavailable"
    return "Suggested Manifest: \(manifestPath)\nSuggested Model: \(modelPath)\nSuggested Binary: \(binaryPath)"
  }

  static func artifactPathSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let modelHealth = snapshot.modelHealth else {
      return "No local model paths available yet."
    }

    let modelPath = modelHealth.modelPath ?? "model path unavailable"
    let binaryPath = modelHealth.binaryPath ?? "binary path unavailable"
    let manifestPath = modelHealth.manifestPath ?? "manifest path unavailable"
    return "Model: \(modelPath)\nBinary: \(binaryPath)\nManifest: \(manifestPath)"
  }

  static func shouldShowDownloadProgress(_ snapshot: LocalModelStatusSnapshot) -> Bool {
    guard let progress = snapshot.modelDownloadProgress else {
      return false
    }

    return snapshot.modelDownloadID == progress.modelID
      || snapshot.pausedModelDownloadID == progress.modelID
  }

  static func downloadProgressValue(_ snapshot: LocalModelStatusSnapshot) -> Double? {
    guard let progress = snapshot.modelDownloadProgress,
          progress.totalBytes > 0
    else {
      return nil
    }

    let value = Double(progress.bytesReceived) / Double(progress.totalBytes)
    return min(max(value, 0), 1)
  }

  static func downloadProgressSummary(_ snapshot: LocalModelStatusSnapshot) -> String {
    guard let progress = snapshot.modelDownloadProgress else {
      return ""
    }

    let received = formattedByteCount(progress.bytesReceived)
    let total = progress.totalBytes > 0
      ? formattedByteCount(progress.totalBytes)
      : "unknown size"
    let isPaused = snapshot.pausedModelDownloadID == progress.modelID
    let status = isPaused ? "Paused" : (progress.isResuming ? "Continuing" : "Downloading")
    let trailingStatus: String
    if isPaused {
      trailingStatus = "Ready to continue"
    } else {
      let eta = downloadETASummary(progress).map { " | \($0)" } ?? ""
      trailingStatus = "\(downloadSpeedSummary(progress))\(eta)"
    }
    let percent = downloadProgressValue(snapshot)
      .map { " | \(Int($0 * 100))%" }
      ?? ""

    return "\(status) \(progress.displayName): \(received) of \(total)\(percent) | \(trailingStatus)"
  }

  static func localModelStatusSummary(
    _ model: LocalModelSummary,
    snapshot: LocalModelStatusSnapshot
  ) -> String {
    let status: String
    if snapshot.modelDownloadID == model.id {
      status = "downloading"
    } else if snapshot.pausedModelDownloadID == model.id {
      status = "paused"
    } else if model.active {
      status = "active"
    } else if model.downloaded {
      status = "downloaded"
    } else {
      status = "available"
    }

    let localSize = model.localSizeBytes.map(formattedByteCount) ?? formattedByteCount(model.sizeBytes)
    return "\(status) | \(localSize) | \(model.license)"
  }

  static func defaultDownloadButtonTitle(_ snapshot: LocalModelStatusSnapshot) -> String {
    let setupModelID = snapshot.selectedSetupModel?.id ?? snapshot.selectedSetupModelID
    if snapshot.modelDownloadID == setupModelID {
      return "Downloading Model"
    }
    if snapshot.pausedModelDownloadID == setupModelID {
      return "Continue Model"
    }
    if let setupModel = snapshot.selectedSetupModel {
      if setupModel.active {
        return "Model Selected"
      }
      if setupModel.downloaded {
        return "Use Downloaded Model"
      }
    }

    return "Download Model"
  }

  static func downloadButtonTitle(
    _ model: LocalModelSummary,
    snapshot: LocalModelStatusSnapshot
  ) -> String {
    if snapshot.modelDownloadID == model.id {
      return "Downloading"
    }
    if snapshot.pausedModelDownloadID == model.id {
      return "Continue"
    }

    return model.downloaded ? "Downloaded" : "Download"
  }

  static func tagSummary(_ model: LocalModelSummary) -> String {
    model.tags.joined(separator: " / ")
  }

  static func pathSummary(_ model: LocalModelSummary) -> String {
    model.installPath
  }

  private static func formattedByteCount(_ byteCount: Int64) -> String {
    let formatter = ByteCountFormatter()
    formatter.countStyle = .file
    return formatter.string(fromByteCount: byteCount)
  }

  private static func downloadSpeedSummary(_ progress: ModelDownloadProgress) -> String {
    let elapsed = max(progress.updatedAt.timeIntervalSince(progress.startedAt), 1)
    let bytesPerSecond = Int64(Double(progress.bytesReceived) / elapsed)
    return "\(formattedByteCount(bytesPerSecond))/s"
  }

  private static func downloadETASummary(_ progress: ModelDownloadProgress) -> String? {
    guard progress.bytesReceived > 0,
          progress.totalBytes > progress.bytesReceived
    else {
      return nil
    }

    let elapsed = max(progress.updatedAt.timeIntervalSince(progress.startedAt), 1)
    let bytesPerSecond = Double(progress.bytesReceived) / elapsed
    guard bytesPerSecond > 0 else {
      return nil
    }

    let remainingSeconds = Double(progress.totalBytes - progress.bytesReceived) / bytesPerSecond
    return "ETA \(formattedDuration(remainingSeconds))"
  }

  private static func formattedDuration(_ seconds: TimeInterval) -> String {
    let roundedSeconds = max(Int(seconds.rounded()), 0)
    if roundedSeconds < 60 {
      return "\(roundedSeconds)s"
    }

    let minutes = roundedSeconds / 60
    if minutes < 60 {
      return "\(minutes)m"
    }

    let hours = minutes / 60
    let remainingMinutes = minutes % 60
    if remainingMinutes == 0 {
      return "\(hours)h"
    }

    return "\(hours)h \(remainingMinutes)m"
  }
}
