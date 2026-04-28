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
