import Foundation

@MainActor
final class LocalModelDownloadCoordinator {
  private(set) var task: Task<Void, Never>?
  private var transfer: ModelDownloadTransfer?
  var resumeData: Data?

  init(resumeData: Data? = nil) {
    self.resumeData = resumeData
  }

  var isDownloading: Bool {
    task != nil
  }

  var canPause: Bool {
    task != nil
  }

  func start(_ task: Task<Void, Never>) {
    self.task = task
  }

  func attachTransfer(_ transfer: ModelDownloadTransfer) {
    self.transfer = transfer
  }

  func pauseActiveTransfer() {
    transfer?.pause()
  }

  func cancelActiveDownload() {
    task?.cancel()
    transfer?.cancel()
  }

  func finishActiveDownload() {
    task = nil
    transfer = nil
  }

  func clearResumeData() {
    resumeData = nil
  }
}

struct LocalModelDownloadRuntimeState: Hashable {
  var activeModelID: String?
  var pausedModelID: String?
  var progress: ModelDownloadProgress?

  var hasActiveDownload: Bool {
    activeModelID != nil
  }

  var hasPausedDownload: Bool {
    pausedModelID != nil
  }

  var hasAnyDownloadState: Bool {
    hasActiveDownload || hasPausedDownload
  }

  mutating func applyStart(_ sessionState: LocalModelDownloadSessionStartState) {
    activeModelID = sessionState.activeModelID
    pausedModelID = sessionState.pausedModelID
    progress = sessionState.progress
  }

  mutating func clearActiveDownload() {
    activeModelID = nil
  }

  mutating func markPaused(modelID: String) {
    pausedModelID = modelID
  }

  mutating func clearPausedDownload() {
    pausedModelID = nil
  }

  mutating func clearProgress() {
    progress = nil
  }

  mutating func updateProgress(_ nextProgress: ModelDownloadProgress) {
    progress = nextProgress
  }
}
