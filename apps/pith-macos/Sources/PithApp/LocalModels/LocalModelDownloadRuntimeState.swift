import Foundation

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
