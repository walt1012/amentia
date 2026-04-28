import Foundation

struct LocalModelDownloadRequestCacheKey: Hashable {
  let modelID: String
  let modelDownloaded: Bool
  let modelSizeBytes: Int64
  let modelInstallPath: String
  let isDownloadRunning: Bool
  let pausedModelID: String?
  let hasResumeData: Bool
  let resumeBytesReceived: Int64?
}

struct LocalModelDownloadRequestCache {
  let key: LocalModelDownloadRequestCacheKey
  let createdAt: Date
  let plan: LocalModelDownloadRequestPlan
}

final class LocalModelDownloadRequestPlanCache {
  private let lifetime: TimeInterval
  private var cachedPlan: LocalModelDownloadRequestCache?

  init(lifetime: TimeInterval = 2) {
    self.lifetime = lifetime
  }

  func plan(
    for model: LocalModelSummary,
    isDownloadRunning: Bool,
    pausedModelID: String?,
    resumeData: Data?,
    currentProgress: ModelDownloadProgress?
  ) -> LocalModelDownloadRequestPlan {
    let resumeBytesReceived = Self.resumeBytesReceived(
      for: model.id,
      pausedModelID: pausedModelID,
      currentProgress: currentProgress
    )
    let key = LocalModelDownloadRequestCacheKey(
      modelID: model.id,
      modelDownloaded: model.downloaded,
      modelSizeBytes: model.sizeBytes,
      modelInstallPath: model.installPath,
      isDownloadRunning: isDownloadRunning,
      pausedModelID: pausedModelID,
      hasResumeData: resumeData != nil,
      resumeBytesReceived: resumeBytesReceived
    )
    let now = Date()
    if let cachedPlan,
       cachedPlan.key == key,
       now.timeIntervalSince(cachedPlan.createdAt) < lifetime
    {
      return cachedPlan.plan
    }

    let plan = LocalModelDownloadRequestPlanner.plan(
      model: model,
      isDownloadRunning: key.isDownloadRunning,
      pausedModelID: key.pausedModelID,
      hasResumeData: key.hasResumeData,
      resumeBytesReceived: key.resumeBytesReceived
    )
    cachedPlan = LocalModelDownloadRequestCache(
      key: key,
      createdAt: now,
      plan: plan
    )
    return plan
  }

  func clear() {
    cachedPlan = nil
  }

  private static func resumeBytesReceived(
    for modelID: String,
    pausedModelID: String?,
    currentProgress: ModelDownloadProgress?
  ) -> Int64? {
    guard pausedModelID == modelID,
          let currentProgress,
          currentProgress.modelID == modelID
    else {
      return nil
    }

    return currentProgress.bytesReceived
  }
}
