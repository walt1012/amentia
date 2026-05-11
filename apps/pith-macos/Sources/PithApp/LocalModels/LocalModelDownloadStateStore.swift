import Foundation

@MainActor
enum LocalModelDownloadStateStore {
  static func clearPausedDownload(coordinator: LocalModelDownloadCoordinator) {
    coordinator.clearResumeData()
    LocalModelCatalog.clearPausedDownload()
  }

  static func persistPausedDownload(
    modelID: String,
    resumeData: Data,
    progress: ModelDownloadProgress?,
    updatedAt: Date = Date()
  ) {
    guard !resumeData.isEmpty else {
      return
    }

    LocalModelCatalog.savePausedDownload(
      modelID: modelID,
      resumeData: resumeData,
      bytesReceived: progress?.bytesReceived ?? 0,
      totalBytes: progress?.totalBytes ?? 0,
      updatedAt: progress?.updatedAt ?? updatedAt
    )
  }

  static func removeIncompleteModelFile(modelID: String, models: [LocalModelSummary]) {
    guard let model = models.first(where: { $0.id == modelID }) else {
      return
    }

    let targetURL = URL(fileURLWithPath: model.installPath)
    let manager = FileManager.default
    guard manager.fileExists(atPath: targetURL.path) else {
      return
    }

    if (try? LocalModelCatalog.validateDownloadedModel(model)) != nil {
      return
    }

    try? manager.removeItem(at: targetURL)
  }
}
