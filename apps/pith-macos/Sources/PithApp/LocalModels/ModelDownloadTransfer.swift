import Foundation

struct ModelDownloadPaused: Error {
  let resumeData: Data
}

final class ModelDownloadTransfer: NSObject, URLSessionDownloadDelegate {
  private let targetURL: URL
  private let onProgress: (Int64, Int64) -> Void
  private var continuation: CheckedContinuation<Void, Error>?
  private var session: URLSession?
  private var task: URLSessionDownloadTask?
  private var pauseRequested = false

  init(targetURL: URL, onProgress: @escaping (Int64, Int64) -> Void) {
    self.targetURL = targetURL
    self.onProgress = onProgress
  }

  func start(from sourceURL: URL, resumeData: Data?) async throws {
    try await withTaskCancellationHandler {
      try await withCheckedThrowingContinuation { continuation in
        self.continuation = continuation
        let session = URLSession(configuration: .default, delegate: self, delegateQueue: nil)
        self.session = session
        let task = resumeData.map { session.downloadTask(withResumeData: $0) }
          ?? session.downloadTask(with: sourceURL)
        self.task = task
        task.resume()
      }
    } onCancel: {
      self.cancel()
    }
  }

  func pause() {
    pauseRequested = true
    task?.cancel(byProducingResumeData: { [weak self] resumeData in
      guard let self else {
        return
      }

      guard let resumeData, !resumeData.isEmpty else {
        self.complete(.failure(CancellationError()))
        return
      }

      self.complete(.failure(ModelDownloadPaused(resumeData: resumeData)))
    })
  }

  func cancel() {
    pauseRequested = false
    task?.cancel()
  }

  func urlSession(
    _ session: URLSession,
    downloadTask: URLSessionDownloadTask,
    didWriteData _: Int64,
    totalBytesWritten: Int64,
    totalBytesExpectedToWrite: Int64
  ) {
    onProgress(totalBytesWritten, totalBytesExpectedToWrite)
  }

  func urlSession(
    _ session: URLSession,
    downloadTask: URLSessionDownloadTask,
    didFinishDownloadingTo location: URL
  ) {
    if let httpResponse = downloadTask.response as? HTTPURLResponse,
       !(200..<300).contains(httpResponse.statusCode)
    {
      complete(
        .failure(
          NSError(
            domain: "PithModelDownload",
            code: httpResponse.statusCode,
            userInfo: [
              NSLocalizedDescriptionKey:
                "Model download failed with HTTP \(httpResponse.statusCode)."
            ]
          )
        )
      )
      return
    }

    do {
      let manager = FileManager.default
      try manager.createDirectory(
        at: targetURL.deletingLastPathComponent(),
        withIntermediateDirectories: true
      )

      if manager.fileExists(atPath: targetURL.path) {
        try manager.removeItem(at: targetURL)
      }

      try manager.moveItem(at: location, to: targetURL)
      complete(.success(()))
    } catch {
      complete(.failure(error))
    }
  }

  func urlSession(
    _ session: URLSession,
    task: URLSessionTask,
    didCompleteWithError error: Error?
  ) {
    guard let error else {
      return
    }

    if pauseRequested {
      if let resumeData = (error as NSError).userInfo[NSURLSessionDownloadTaskResumeData] as? Data,
         !resumeData.isEmpty
      {
        complete(.failure(ModelDownloadPaused(resumeData: resumeData)))
      } else {
        complete(.failure(CancellationError()))
      }
      return
    }

    complete(.failure(error))
  }

  private func complete(_ result: Result<Void, Error>) {
    guard let continuation else {
      return
    }

    self.continuation = nil
    task = nil
    session?.invalidateAndCancel()
    session = nil

    switch result {
    case .success:
      continuation.resume()
    case .failure(let error):
      continuation.resume(throwing: error)
    }
  }
}
