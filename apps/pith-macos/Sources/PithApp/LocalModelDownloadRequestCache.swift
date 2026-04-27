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
