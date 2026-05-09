import Foundation

enum LocalModelDownloadStatusPresenter {
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

    let received = LocalModelByteFormatter.string(progress.bytesReceived)
    let total = progress.totalBytes > 0
      ? LocalModelByteFormatter.string(progress.totalBytes)
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

  private static func downloadSpeedSummary(_ progress: ModelDownloadProgress) -> String {
    let elapsed = max(progress.updatedAt.timeIntervalSince(progress.startedAt), 1)
    let bytesPerSecond = Int64(Double(progress.bytesReceived) / elapsed)
    return "\(LocalModelByteFormatter.string(bytesPerSecond))/s"
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
