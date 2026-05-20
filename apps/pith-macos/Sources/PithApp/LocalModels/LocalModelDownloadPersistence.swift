import Foundation

struct ModelDownloadProgress: Hashable, Sendable {
  let modelID: String
  let displayName: String
  var bytesReceived: Int64
  var totalBytes: Int64
  let startedAt: Date
  var updatedAt: Date
  let isResuming: Bool
}

struct PersistedModelDownload {
  let modelID: String
  let resumeData: Data
  let bytesReceived: Int64
  let totalBytes: Int64
  let updatedAt: Date
}

extension LocalModelCatalog {
  private static let pausedDownloadIDKey = "pith.pausedModelDownloadID"
  private static let pausedDownloadBytesReceivedKey = "pith.pausedModelDownloadBytesReceived"
  private static let pausedDownloadTotalBytesKey = "pith.pausedModelDownloadTotalBytes"
  private static let pausedDownloadUpdatedAtKey = "pith.pausedModelDownloadUpdatedAt"

  static func loadPausedDownload(matching localModels: [LocalModelSummary]) -> PersistedModelDownload? {
    let defaults = UserDefaults.standard
    guard let modelID = defaults.string(forKey: pausedDownloadIDKey),
          localModels.contains(where: { $0.id == modelID }),
          let resumeData = try? Data(contentsOf: pausedDownloadResumeDataURL()),
          !resumeData.isEmpty
    else {
      clearPausedDownload()
      return nil
    }

    let bytesReceived = max(Int64(defaults.integer(forKey: pausedDownloadBytesReceivedKey)), 0)
    let totalBytes = max(Int64(defaults.integer(forKey: pausedDownloadTotalBytesKey)), 0)
    let updatedAt = defaults.object(forKey: pausedDownloadUpdatedAtKey) as? Date ?? Date()
    return PersistedModelDownload(
      modelID: modelID,
      resumeData: resumeData,
      bytesReceived: bytesReceived,
      totalBytes: totalBytes,
      updatedAt: updatedAt
    )
  }

  static func restoredProgress(
    from pausedDownload: PersistedModelDownload?,
    localModels: [LocalModelSummary]
  ) -> ModelDownloadProgress? {
    guard let pausedDownload,
          let model = localModels.first(where: { $0.id == pausedDownload.modelID })
    else {
      return nil
    }

    return ModelDownloadProgress(
      modelID: model.id,
      displayName: model.displayName,
      bytesReceived: pausedDownload.bytesReceived,
      totalBytes: pausedDownload.totalBytes > 0 ? pausedDownload.totalBytes : model.sizeBytes,
      startedAt: pausedDownload.updatedAt,
      updatedAt: pausedDownload.updatedAt,
      isResuming: true
    )
  }

  static func savePausedDownload(
    modelID: String,
    resumeData: Data,
    bytesReceived: Int64,
    totalBytes: Int64,
    updatedAt: Date
  ) {
    let resumeDataURL = pausedDownloadResumeDataURL()
    let manager = FileManager.default
    do {
      try manager.createDirectory(
        at: resumeDataURL.deletingLastPathComponent(),
        withIntermediateDirectories: true
      )
      try resumeData.write(to: resumeDataURL, options: .atomic)
    } catch {
      clearPausedDownload()
      return
    }

    let defaults = UserDefaults.standard
    defaults.set(modelID, forKey: pausedDownloadIDKey)
    defaults.set(max(bytesReceived, 0), forKey: pausedDownloadBytesReceivedKey)
    defaults.set(max(totalBytes, 0), forKey: pausedDownloadTotalBytesKey)
    defaults.set(updatedAt, forKey: pausedDownloadUpdatedAtKey)
  }

  static func clearPausedDownload() {
    try? FileManager.default.removeItem(at: pausedDownloadResumeDataURL())
    let defaults = UserDefaults.standard
    defaults.removeObject(forKey: pausedDownloadIDKey)
    defaults.removeObject(forKey: pausedDownloadBytesReceivedKey)
    defaults.removeObject(forKey: pausedDownloadTotalBytesKey)
    defaults.removeObject(forKey: pausedDownloadUpdatedAtKey)
  }

  private static func pausedDownloadResumeDataURL() -> URL {
    AppSupportDirectories.modelDownloadDirectory()
      .appendingPathComponent("resume.data")
  }
}
