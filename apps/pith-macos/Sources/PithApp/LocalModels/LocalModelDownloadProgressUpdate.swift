import Foundation

struct LocalModelDownloadProgressUpdate {
  let modelID: String
  let activeModelID: String?
  let currentProgress: ModelDownloadProgress?
  let bytesReceived: Int64
  let totalBytes: Int64
  let updatedAt: Date
}

enum LocalModelDownloadProgressUpdater {
  static func updatedProgress(
    _ update: LocalModelDownloadProgressUpdate
  ) -> ModelDownloadProgress? {
    guard update.activeModelID == update.modelID,
          update.currentProgress?.modelID == update.modelID,
          var progress = update.currentProgress
    else {
      return nil
    }

    progress.bytesReceived = max(update.bytesReceived, progress.bytesReceived)
    progress.totalBytes = update.totalBytes > 0 ? update.totalBytes : progress.totalBytes
    progress.updatedAt = update.updatedAt
    return progress
  }
}
